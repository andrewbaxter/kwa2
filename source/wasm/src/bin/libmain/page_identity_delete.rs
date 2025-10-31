use {
    crate::libmain::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
            get_stored_api_identities,
            req_api_identities,
            LocalIdentity,
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
    shared::interface::wire::c2s::{
        self,
    },
    spaghettinuum::interface::identity::Identity,
    wasm::js::{
        el_async,
        style_export,
    },
};

pub fn build1(eg: EventGraph, local: LocalIdentity) -> El {
    return build_form(
        format!("Delete identity"),
        Ministate::Identity(local.res.id.clone()),
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
    );
}

pub fn build(pc: &mut ProcessingContext, id: &Identity) -> El {
    match get_stored_api_identities(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(local) => {
            return build1(pc.eg(), local);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let id = id.clone();
                async move {
                    let Some(local) =
                        req_api_identities(Some(&id)).await?.into_iter().find(|x| x.res.id == id) else {
                            return Err(format!("Could not find identity [{:?}]", id));
                        };
                    return Ok(vec![build1(eg.clone(), local)]);
                }
            });
        },
    }
}
