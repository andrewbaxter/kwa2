use {
    crate::{
        api::req_post_json,
        js::style_export,
        localdata::{
            self,
            get_or_req_api_channelinvite,
        },
        pageutil::build_nol_form,
        state::{
            goto_replace_ministate,
            state,
            Ministate,
            MinistateChannel,
            MinistateChannelInvite,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        el,
        El,
    },
    shared::interface::{
        shared::{
            ChannelInviteId,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::{
            self,
        },
    },
};

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
    }), "Delete invite", get_or_req_api_channelinvite(id, false).map({
        let eg = pc.eg();
        let reset_id = reset_id.clone();
        move |local| (
            el("div"),
            vec![
                style_export::leaf_form_text(
                    style_export::LeafFormTextArgs {
                        text: format!("Are you sure you want to delete invite [{}]", local.res.memo_short),
                    },
                ).root
            ],
            async move |_idem| {
                req_post_json(&state().env.base_url, c2s::ChannelInviteDelete { id: local.res.id.clone() }).await?;
                localdata::delete_channelinvite(local.res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelInvites(MinistateChannel {
                        channel: local.res.token.channel.clone(),
                        reset: reset_id.clone(),
                    }));
                }).unwrap();
                return Ok(());
            },
        )
    }));
}
