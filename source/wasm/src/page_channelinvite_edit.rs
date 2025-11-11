use {
    crate::{
        api::req_post_json,
        localdata::{
            self,
            get_or_req_api_channelinvite,
        },
        pageutil::build_nol_form,
        state::{
            goto_replace_ministate,
            state,
            Ministate,
            MinistateChannelInvite,
        },
    },
    jiff::Timestamp,
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        shared::{
            ChannelInviteId,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::{
            self,
            ModifyOption,
        },
    },
    std::rc::Rc,
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

pub fn build(
    pc: &mut ProcessingContext,
    channel: &QualifiedChannelId,
    id: &ChannelInviteId,
    reset_id: &Option<QualifiedMessageId>,
) -> El {
    return build_nol_form(&Ministate::ChannelInvite(MinistateChannelInvite {
        channel: channel.clone(),
        reset: reset_id.clone(),
        invite: id.clone(),
    }), "Edit invite", get_or_req_api_channelinvite(id, true).map({
        let eg = pc.eg();
        let reset_id = reset_id.clone();
        let channel = channel.clone();
        move |local| {
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                memo_short: local.res.memo_short.clone(),
                memo_long: local.res.memo_long.clone(),
                single_use: local.res.single_use.clone(),
                expiry: local.res.expiry.clone(),
            }));
            let form_state = Rc::new(form_state);
            return (form_els.error.unwrap(), form_els.elements, async move |_idem| {
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
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelInvite(MinistateChannelInvite {
                        channel: channel.clone(),
                        invite: res.id,
                        reset: reset_id.clone(),
                    }));
                }).unwrap();
                return Ok(());
            });
        }
    }));
}
