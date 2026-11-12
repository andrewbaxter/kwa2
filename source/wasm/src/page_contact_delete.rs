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
            goto_replace_ministate,
            state,
            Ministate,
            MinistateContact,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    shared::interface::wire::c2s,
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateContact) -> El {
    return build_nol_form(
        pc,
        &Ministate::Contact(s.clone()),
        "Delete contact",
        get_or_req_api_contact(&s.identity, false).map({
            let eg = pc.eg();
            let s = s.clone();
            move |local| (
                None,
                vec![
                    style_export::leaf_form_text(
                        style_export::LeafFormTextArgs {
                            text: format!("Are you sure you want to delete contact [{}]", local.res.name),
                        },
                    ).root,
                ],
                async move |_idem| {
                    req_post_json(&state().env.base_url, c2s::ContactDelete { id: local.res.id.clone() }).await?;
                    localdata::delete_contact(local.res.clone()).await;
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::Contact(s.clone()));
                    }).unwrap();
                    return Ok(());
                },
            )
        }),
    );
}
