use {
    crate::{
        api::req_post_json,
        formutil::{
            build_form,
            FormOptChannelGroup,
        },
        localdata::{
            self,
            get_stored_api_channels,
            req_api_channels,
            LocalChannel,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
        },
    },
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::{
        c2s::{
            self,
        },
        shared::QualifiedChannelId,
    },
    std::rc::Rc,
    crate::js::el_async,
};

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Short memo")]
    memo_short: String,
    #[title("Extra memo")]
    memo_long: String,
    #[title("Group")]
    group: FormOptChannelGroup,
}

pub fn build1(eg: EventGraph, value: LocalChannel) -> El {
    let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
        memo_short: value.res.memo_short.clone(),
        memo_long: value.res.memo_long.clone(),
        group: FormOptChannelGroup(value.res.group.clone()),
    }));
    let form_state = Rc::new(form_state);
    return build_form(
        format!("Edit channel"),
        Ministate::Channel(value.res.id.clone()),
        form_els.error.unwrap(),
        form_els.elements,
        async move |_idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            let res = req_post_json(&state().env.base_url, c2s::ChannelModify {
                id: value.res.id.clone(),
                memo_short: if new_values.memo_short == value.res.memo_short {
                    None
                } else {
                    Some(new_values.memo_short)
                },
                memo_long: if new_values.memo_long == value.res.memo_long {
                    None
                } else {
                    Some(new_values.memo_long)
                },
                group: if new_values.group.0 == value.res.group {
                    None
                } else {
                    Some(c2s::ModifyOption { value: new_values.group.0 })
                },
            }).await?;
            localdata::ensure_channel(res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &&Ministate::Channel(res.id));
            }).unwrap();
            return Ok(());
        },
    );
}

pub fn build(pc: &mut ProcessingContext, id: &QualifiedChannelId) -> El {
    match get_stored_api_channels(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(value) => {
            return build1(pc.eg(), value);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let channel = id.clone();
                async move {
                    let Some(value) =
                        req_api_channels(Some(&channel)).await?.into_iter().find(|x| x.res.id == channel) else {
                            return Err(format!("Could not find channel [{:?}]", channel));
                        };
                    return Ok(vec![build1(eg.clone(), value)]);
                }
            });
        },
    }
}
