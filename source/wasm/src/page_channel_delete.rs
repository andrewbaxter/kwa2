use {
    crate::{
        api::req_post_json,
        js::style_export,
        localdata::{
            self,
            get_or_req_api_channel,
        },
        pageutil::build_nol_form,
        state::{
            Ministate,
            MinistateChannel,
            MinistateChannelSub,
            goto_replace_ministate,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        El,
        el,
    },
    shared::interface::{
        wire::c2s::{
            self,
        },
    },
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelSub) -> El {
    return build_nol_form(&Ministate::Channel(MinistateChannel {
        id: s.id.clone(),
        own_identity: s.own_identity.clone(),
        reset_id: None,
    }), "Delete channel", get_or_req_api_channel(&s.id, false).map({
        let eg = pc.eg();
        |local| (
            el("div"),
            vec![
                style_export::leaf_form_text(
                    style_export::LeafFormTextArgs {
                        text: format!("Are you sure you want to delete channel [{}]", local.res.memo_short),
                    },
                ).root
            ],
            async move |_idem| {
                req_post_json(&state().env.base_url, c2s::ChannelDelete { id: local.res.id.clone() }).await?;
                localdata::delete_channel(local.res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::Top);
                }).unwrap();
                return Ok(());
            },
        )
    }));
}
