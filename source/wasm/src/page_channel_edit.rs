use {
    crate::{
        api::req_post_json,
        localdata::{
            self,
            get_or_req_api_channel,
        },
        pageutil::{
            FormOptChannelGroup,
            build_nol_form,
        },
        state::{
            Ministate,
            MinistateChannel,
            MinistateChannelSub,
            goto_replace_ministate,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        wire::c2s::{
            self,
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
    #[title("Group")]
    group: FormOptChannelGroup,
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelSub) -> El {
    return build_nol_form(&Ministate::Channel(MinistateChannel {
        id: s.id.clone(),
        own_identity: s.own_identity.clone(),
        reset_id: None,
    }), "Edit channel", get_or_req_api_channel(&s.id, false).map({
        let eg = pc.eg();
        let own_identity = s.own_identity.clone();
        move |local| {
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                memo_short: local.res.memo_short.clone(),
                memo_long: local.res.memo_long.clone(),
                group: FormOptChannelGroup(local.res.group.clone()),
            }));
            let form_state = Rc::new(form_state);
            (form_els.error.unwrap(), form_els.elements, async move |_idem| {
                let Ok(new_values) = form_state.parse() else {
                    return Ok(());
                };
                let res = req_post_json(&state().env.base_url, c2s::ChannelModify {
                    own_identity: own_identity.clone(),
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
                    group: if new_values.group.0 == local.res.group {
                        None
                    } else {
                        Some(c2s::ModifyOption { value: new_values.group.0 })
                    },
                }).await?;
                localdata::ensure_channel(res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &&Ministate::Channel(MinistateChannel {
                        id: res.id.clone(),
                        own_identity: own_identity.clone(),
                        reset_id: None,
                    }));
                }).unwrap();
                return Ok(());
            })
        }
    }));
}
