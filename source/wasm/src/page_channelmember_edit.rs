use {
    crate::{
        api::req_post_json,
        localdata::{
            self,
            get_or_req_api_contact,
        },
        pageutil::build_nol_form,
        state::{
            Ministate,
            MinistateChannelMember,
            goto_replace_ministate,
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
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelMember) -> El {
    return build_nol_form(&Ministate::ChannelMember(MinistateChannelMember {
        channel: s.channel.clone(),
        identity: s.identity.clone(),
    }), "Edit member", get_or_req_api_contact(&s.identity, true).map({
        let eg = pc.eg();
        let channel = s.channel.clone();
        move |local| {
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                memo_short: local.res.memo_short.clone(),
                memo_long: local.res.memo_long.clone(),
            }));
            let form_state = Rc::new(form_state);
            return (form_els.error.unwrap(), form_els.elements, async move |_idem| {
                let Ok(new_values) = form_state.parse() else {
                    return Ok(());
                };
                let res = req_post_json(&state().env.base_url, c2s::ContactModify {
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
                }).await?;
                localdata::ensure_contact(res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelMember(MinistateChannelMember {
                        channel: channel.clone(),
                        identity: res.id,
                    }));
                }).unwrap();
                return Ok(());
            });
        }
    }));
}
