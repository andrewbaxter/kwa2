use {
    crate::{
        api::req_post_json,
        pageutil::build_nol_form,
        js::style_export,
        localdata::{
            self,
            get_or_req_api_identity,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        el,
        El,
    },
    shared::interface::wire::c2s::{
        self,
    },
    spaghettinuum::interface::identity::Identity,
};

pub fn build(pc: &mut ProcessingContext, id: &Identity) -> El {
    return build_nol_form(
        &Ministate::Identity(id.clone()),
        "Delete identity",
        get_or_req_api_identity(id, false).map({
            let eg = pc.eg();
            move |local| (
                el("div"),
                vec![
                    style_export::leaf_form_text(
                        style_export::LeafFormTextArgs {
                            text: format!("Are you sure you want to delete identity [{}]", local.res.memo_short),
                        },
                    ).root
                ],
                async move |_idem| {
                    req_post_json(&state().env.base_url, c2s::IdentityDelete { id: local.res.id.clone() }).await?;
                    localdata::delete_identity(local.res.clone()).await;
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::Identities);
                    }).unwrap();
                    return Ok(());
                },
            )
        }),
    )
}
