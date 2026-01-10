use {
    crate::{
        api::req_post_json,
        pageutil::{
            FormOptChannelGroup,
            build_nol_form,
        },
        state::{
            Ministate,
            MinistateChannel,
            MinistateChannelSub,
            get_or_req_channel,
            goto_replace_ministate,
            pull_top,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::c2s::{
        self,
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
    return build_nol_form(
        //. .
        pc,&Ministate::Channel(MinistateChannel {
        id: s.id.clone(),
        own_identity: s.own_identity.clone(),
        reset_id: None,
    }), "Edit channel", get_or_req_channel(&pc.eg(), &s.id, false).map({
        let eg = pc.eg();
        let own_identity = s.own_identity.clone();
        move |local| {
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                memo_short: local.memo_short.get(),
                memo_long: local.memo_long.get(),
                group: FormOptChannelGroup(local.group.get()),
            }));
            let form_state = Rc::new(form_state);
            (form_els.error.unwrap(), form_els.elements, async move |_idem| {
                let Ok(new_values) = form_state.parse() else {
                    return Ok(());
                };
                let res = req_post_json(&state().env.base_url, c2s::ChannelModify {
                    own_identity: own_identity.clone(),
                    id: local.id.clone(),
                    memo_short: if new_values.memo_short == *local.memo_short.borrow() {
                        None
                    } else {
                        Some(new_values.memo_short)
                    },
                    memo_long: if new_values.memo_long == *local.memo_long.borrow() {
                        None
                    } else {
                        Some(new_values.memo_long)
                    },
                    group: if new_values.group.0 == *local.group.borrow() {
                        None
                    } else {
                        Some(c2s::ModifyOption { value: new_values.group.0 })
                    },
                }).await?;
                pull_top(&eg).await;
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
