use {
    crate::{
        api::req_post_json,
        js::style_export,
        localdata::{
            self,
            get_or_req_api_contact_opt,
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
    shared::interface::wire::c2s,
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelMember) -> El {
    return build_nol_form(
        //. .
        pc,
        &Ministate::ChannelMember(s.clone()),
        "Delete member",
        get_or_req_api_contact_opt(&s.identity, false).map({
            let eg = pc.eg();
            let s = s.clone();
            move |local| (
                None,
                vec![
                    style_export::leaf_form_text(
                        style_export::LeafFormTextArgs {
                            text: format!("Are you sure you want to delete member [{}]", match &local {
                                Some(local) if !local.res.name.is_empty() => local.res.name.clone(),
                                _ => s.identity.to_string(),
                            }),
                        },
                    ).root,
                ],
                async move |_idem| {
                    req_post_json(&state().env.base_url, c2s::ChannelMemberDelete {
                        channel: s.channel.clone(),
                        member: s.identity.clone(),
                    }).await?;
                    localdata::delete_channelmember(&s.channel, s.identity.clone()).await;
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::ChannelMembers(s.channel.clone()));
                    }).unwrap();
                    return Ok(());
                },
            )
        }),
    );
}
