use {
    crate::{
        api::req_post_json,
        js::style_export,
        pageutil::build_nol_form,
        state::{
            Ministate,
            MinistateChannelSub,
            get_or_req_channel,
            goto_replace_ministate,
            pull_top,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        El,
    },
    shared::interface::wire::c2s::{
        self,
    },
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelSub) -> El {
    return build_nol_form(
        //. .
        pc,
        &Ministate::ChannelMenu(s.clone()),
        "Delete channel",
        get_or_req_channel(&pc.eg(), &s.id, false).map({
            let eg = pc.eg();
            |local| (
                None,
                vec![
                    style_export::leaf_form_text(
                        style_export::LeafFormTextArgs {
                            text: format!("Are you sure you want to delete channel [{}]", &*local.name.borrow()),
                        },
                    ).root
                ],
                async move |_idem| {
                    req_post_json(&state().env.base_url, c2s::ChannelDelete { id: local.id.clone() }).await?;
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
