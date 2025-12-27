use {
    crate::{
        api::req_post_json,
        localdata::{
            get_or_req_api_channelgroup,
        },
        pageutil::build_nol_form,
        state::{
            Ministate,
            MinistateChannelGroup,
            goto_replace_ministate,
            pull_top,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        shared::ChannelGroupId,
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
}

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId) -> El {
    return build_nol_form(&Ministate::ChannelGroup(MinistateChannelGroup {
        id: id.clone(),
        reset_id: None,
    }), "Edit group", get_or_req_api_channelgroup(id, false).map({
        let eg = pc.eg();
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
                pull_top(&eg).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelGroup(MinistateChannelGroup {
                        id: res.id,
                        reset_id: None,
                    }));
                }).unwrap();
                return Ok(());
            });
        }
    }));
}
