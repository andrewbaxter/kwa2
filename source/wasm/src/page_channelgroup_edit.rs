use {
    crate::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
            get_stored_api_channelgroups,
            req_api_channelgroups,
            LocalChannelGroup,
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
        shared::ChannelGroupId,
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
}

pub fn build1(eg: EventGraph, value: LocalChannelGroup) -> El {
    let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
        memo_short: value.res.memo_short.clone(),
        memo_long: value.res.memo_long.clone(),
    }));
    let form_state = Rc::new(form_state);
    return build_form(
        format!("Edit group"),
        Ministate::ChannelGroup(value.res.id.clone()),
        form_els.error.unwrap(),
        form_els.elements,
        async move |_idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            let res = req_post_json(&state().env.base_url, c2s::ChannelGroupModify {
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
            }).await?;
            localdata::ensure_channelgroup(res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::ChannelGroup(res.id));
            }).unwrap();
            return Ok(());
        },
    );
}

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId) -> El {
    match get_stored_api_channelgroups(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(local) => {
            return build1(pc.eg(), local);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let id = id.clone();
                async move {
                    let Some(local) =
                        req_api_channelgroups(Some(&id)).await?.into_iter().find(|x| x.res.id == id) else {
                            return Err(format!("Could not find group [{:?}]", id));
                        };
                    return Ok(vec![build1(eg.clone(), local)]);
                }
            });
        },
    }
}
