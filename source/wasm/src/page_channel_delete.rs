use {
    crate::{
        api::req_post_json,
        pageutil::build_nol_form,
        js::style_export,
        localdata::{
            self,
            get_or_req_api_channel,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
            MinistateChannel,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        el,
        El,
    },
    shared::interface::{
        wire::{
            c2s::{
                self,
            },
        },
        shared::{
            QualifiedChannelId,
            QualifiedMessageId,
        },
    },
};

pub fn build(pc: &mut ProcessingContext, id: &QualifiedChannelId, reset_id: &Option<QualifiedMessageId>) -> El {
    return build_nol_form(&Ministate::Channel(MinistateChannel {
        channel: id.clone(),
        reset: reset_id.clone(),
    }), "Delete channel", get_or_req_api_channel(id, false).map({
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
