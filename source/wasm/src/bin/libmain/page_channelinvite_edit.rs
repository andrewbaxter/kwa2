use {
    crate::libmain::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
            get_stored_api_channelinvites,
            req_api_channelinvites,
            LocalChannelInvite,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
        },
    },
    jiff::Timestamp,
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::{
        c2s::{
            self,
            ModifyOption,
        },
        shared::ChannelInviteId,
    },
    std::rc::Rc,
    wasm::js::el_async,
};

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Short memo")]
    memo_short: String,
    #[title("Extra memo")]
    memo_long: String,
    #[title("Single use")]
    single_use: bool,
    #[title("Expiry")]
    expiry: Option<Timestamp>,
}

pub fn build1(eg: EventGraph, local: LocalChannelInvite) -> El {
    let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
        memo_short: local.res.memo_short.clone(),
        memo_long: local.res.memo_long.clone(),
        single_use: local.res.single_use.clone(),
        expiry: local.res.expiry.clone(),
    }));
    let form_state = Rc::new(form_state);
    return build_form(
        format!("Edit invite"),
        Ministate::ChannelInvite(local.res.id),
        form_els.error.unwrap(),
        form_els.elements,
        async move |_idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            let res = req_post_json(&state().env.base_url, c2s::ChannelInviteModify {
                id: local.res.id.clone(),
                memo_short: if new_values.memo_short == local.res.memo_short {
                    None
                } else {
                    Some(new_values.memo_short)
                },
                memo_long: if new_values.memo_long == local.res.memo_long {
                    None
                } else {
                    Some(new_values.memo_long)
                },
                single_use: if new_values.single_use == local.res.single_use {
                    None
                } else {
                    Some(new_values.single_use)
                },
                expiry: if new_values.expiry == local.res.expiry {
                    None
                } else {
                    Some(ModifyOption { value: new_values.expiry })
                },
            }).await?;
            localdata::ensure_channelinvite(res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::ChannelInvite(res.id));
            }).unwrap();
            return Ok(());
        },
    );
}

pub fn build(pc: &mut ProcessingContext, id: &ChannelInviteId) -> El {
    match get_stored_api_channelinvites(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(local) => {
            return build1(pc.eg(), local);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let invite = id.clone();
                async move {
                    let Some(local) =
                        req_api_channelinvites(Some(&invite))
                            .await?
                            .into_iter()
                            .find(|x| x.res.id == invite) else {
                            return Err(format!("Could not find channel invite [{}]", invite.0));
                        };
                    return Ok(vec![build1(eg.clone(), local)]);
                }
            });
        },
    }
}
