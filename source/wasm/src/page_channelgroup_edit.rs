use {
    crate::{
        api::req_post_json,
        pageutil::build_nol_form,
        localdata::{
            self,
            get_or_req_api_channelgroup,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
            MinistateChannelGroup,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        wire::{
            c2s::{
                self,
            },
        },
        shared::{
            ChannelGroupId,
            QualifiedMessageId,
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
}

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId, reset_id: &Option<QualifiedMessageId>) -> El {
    return build_nol_form(&Ministate::ChannelGroup(MinistateChannelGroup {
        channelgroup: id.clone(),
        reset: reset_id.clone(),
    }), "Edit group", get_or_req_api_channelgroup(id, false).map({
        let eg = pc.eg();
        let reset_id = reset_id.clone();
        move |value| {
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                memo_short: value.res.memo_short.clone(),
                memo_long: value.res.memo_long.clone(),
            }));
            let form_state = Rc::new(form_state);
            return (form_els.error.unwrap(), form_els.elements, async move |_idem| {
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
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelGroup(MinistateChannelGroup {
                        channelgroup: res.id,
                        reset: reset_id.clone(),
                    }));
                }).unwrap();
                return Ok(());
            });
        }
    }));
}
