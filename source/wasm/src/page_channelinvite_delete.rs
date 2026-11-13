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
            MinistateChannelInvite,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        El,
    },
    shared::interface::{
        shared::{
            ChannelInviteId,
            QualifiedChannelId,
        },
        wire::c2s::{
            self,
        },
    },
};

pub fn build(pc: &mut ProcessingContext, channel: &QualifiedChannelId, id: &ChannelInviteId) -> El {
    return build_nol_form(pc, &Ministate::ChannelInvite(MinistateChannelInvite {
        channel: channel.clone(),
        invite: id.clone(),
    }), "Delete invite", get_or_req_api_channelinvite(channel, id, false).map({
        let eg = pc.eg();
        move |local| (
            None,
            vec![
                style_export::leaf_form_text(
                    style_export::LeafFormTextArgs {
                        text: format!("Are you sure you want to delete invite [{}]", local.res.name),
                    },
                ).root,
            ],
            async move |_idem| {
                req_post_json(&state().env.base_url, c2s::ChannelInviteDelete {
                    channel: local.res.token.channel.clone(),
                    id: local.res.id.clone(),
                }).await?;
                localdata::delete_channelinvite(local.res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(
                        pc,
                        &state().log,
                        &Ministate::ChannelInvites(local.res.token.channel.clone()),
                    );
                }).unwrap();
                return Ok(());
            },
        )
    }));
}
