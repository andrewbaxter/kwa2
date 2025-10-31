use {
    crate::libmain::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
            get_stored_api_identityinvites,
            req_api_identityinvites,
            LocalIdentityInvite,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
        },
    },
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::{
        el,
        El,
    },
    shared::interface::wire::{
        c2s::{
            self,
        },
        shared::IdentityInviteId,
    },
    wasm::js::{
        el_async,
        style_export,
    },
};

pub fn build1(eg: EventGraph, local: LocalIdentityInvite) -> El {
    return build_form(
        format!("Delete invite"),
        Ministate::IdentityInvite(local.res.id.clone()),
        el("div"),
        vec![
            style_export::leaf_form_text(
                style_export::LeafFormTextArgs {
                    text: format!("Are you sure you want to delete invite [{}]", local.res.memo_short),
                },
            ).root
        ],
        async move |_idem| {
            req_post_json(&state().env.base_url, c2s::IdentityInviteDelete { id: local.res.id.clone() }).await?;
            localdata::delete_identityinvite(local.res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(
                    pc,
                    &state().log,
                    &Ministate::IdentityInvites(local.res.token.identity.clone()),
                );
            }).unwrap();
            return Ok(());
        },
    );
}

pub fn build(pc: &mut ProcessingContext, id: &IdentityInviteId) -> El {
    match get_stored_api_identityinvites(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(local) => {
            return build1(pc.eg(), local);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let id = id.clone();
                async move {
                    let Some(local) =
                        req_api_identityinvites(Some(&id)).await?.into_iter().find(|x| x.res.id == id) else {
                            return Err(format!("Could not find invite [{:?}]", id));
                        };
                    return Ok(vec![build1(eg.clone(), local)]);
                }
            });
        },
    }
}
