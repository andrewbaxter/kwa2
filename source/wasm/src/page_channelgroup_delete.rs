use {
    crate::{
        api::req_post_json,
        js::style_export,
        pageutil::build_nol_form,
        state::{
            Ministate,
            get_or_req_channelgroup,
            goto_replace_ministate,
            pull_top,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        El,
    },
    shared::interface::{
        shared::ChannelGroupId,
        wire::c2s::{
            self,
        },
    },
};

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId) -> El {
    return build_nol_form(
        //. .
        pc,
        &Ministate::ChannelGroupMenu(id.clone()),
        "Delete group",
        get_or_req_channelgroup(&pc.eg(), id, false).map({
            let eg = pc.eg();
            move |local| (
                None,
                vec![
                    style_export::leaf_form_text(
                        style_export::LeafFormTextArgs {
                            text: format!("Are you sure you want to delete group [{}]", &*local.name.borrow()),
                        },
                    ).root,
                ],
                async move |_idem| {
                    req_post_json(&state().env.base_url, c2s::ChannelGroupDelete { id: local.id.clone() }).await?;
                    pull_top(&eg).await;
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::Top);
                    }).unwrap();
                    return Ok(());
                },
            )
        }),
    );
}
