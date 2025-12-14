use {
    crate::{
        api::req_post_json,
        js::style_export,
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
    rooting::{
        El,
        el,
    },
    shared::interface::wire::c2s::{
        self,
    },
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelMember) -> El {
    return build_nol_form(&Ministate::ChannelMember(MinistateChannelMember {
        channel: s.channel.clone(),
        identity: s.identity.clone(),
    }), "Delete member", get_or_req_api_contact(&s.identity, false).map({
        let eg = pc.eg();
        let channel = s.channel.clone();
        move |local| (
            el("div"),
            vec![
                style_export::leaf_form_text(
                    style_export::LeafFormTextArgs {
                        text: format!("Are you sure you want to delete member [{}]", local.res.memo_short),
                    },
                ).root
            ],
            async move |_idem| {
                req_post_json(&state().env.base_url, c2s::ChannelMemberDelete {
                    channel: channel.clone(),
                    member: local.res.id.clone(),
                }).await?;
                localdata::delete_channelmember(&channel, local.res.id.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelMembers(channel.clone()));
                }).unwrap();
                return Ok(());
            },
        )
    }));
}
