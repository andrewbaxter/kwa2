use {
    crate::{
        api::req_post_json,
        js::style_export,
        localdata::{
            self,
            get_or_req_api_identity,
        },
        pageutil::{
            build_nol_edit_page,
            req_form_portrait,
            EditPage,
            FormPortrait,
        },
        state::{
            build_ministate,
            ministate_octothorpe,
            state,
            Ministate,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::c2s::{
        self,
        ModifyOption,
    },
    spaghettinuum::interface::identity::Identity,
    std::rc::Rc,
};

//# genemichaels-norm-skip
#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Portrait")]
    portrait: FormPortrait,
    #[title("Name")]
    name: String,
    #[title("Shown name")]
    shown_name: Option<String>,
    #[title("Description")]
    description: String,
    #[title("Shown description")]
    shown_description: Option<String>,
}

pub fn build(pc: &mut ProcessingContext, identity: &Identity) -> El {
    return build_nol_edit_page(pc, &Ministate::Identities, get_or_req_api_identity(identity, true).map_async({
        let eg = pc.eg();
        async move |local| {
            let local = local.res;
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                name: local.name.clone(),
                description: local.description.clone(),
                shown_name: local.shown_name.clone(),
                shown_description: local.shown_description.clone(),
                portrait: req_form_portrait(&local.portrait).await?,
            }));
            let form_state = Rc::new(form_state);
            return Ok(EditPage {
                center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                    text: local.name.clone(),
                    link: None,
                }).root,
                submit_text: format!("Save"),
                form_error: form_els.error,
                form: form_els.elements,
                menu: vec![style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Invitations"),
                    link: ministate_octothorpe(&Ministate::IdentityInvites(local.id.clone())),
                    image: None,
                    generated_image: None,
                }).root, style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::IdentityDelete(local.id.clone())),
                    image: None,
                    generated_image: None,
                }).root],
                send: async move |_idem| {
                    let Ok(new_values) = form_state.parse() else {
                        return Ok(());
                    };
                    let res = req_post_json(&state().env.base_url, c2s::IdentityModify {
                        id: local.id.clone(),
                        name: if new_values.name == local.name {
                            None
                        } else {
                            Some(new_values.name)
                        },
                        description: if new_values.description == local.description {
                            None
                        } else {
                            Some(new_values.description)
                        },
                        shown_name: if new_values.shown_name == local.shown_name {
                            None
                        } else {
                            Some(ModifyOption { value: new_values.shown_name })
                        },
                        shown_description: if new_values.shown_description == local.shown_description {
                            None
                        } else {
                            Some(ModifyOption { value: new_values.shown_description })
                        },
                        portrait: new_values.portrait.into_modify().await?,
                    }).await?;
                    localdata::ensure_identity(res.clone()).await;
                    eg.event(|pc| {
                        build_ministate(pc, &Ministate::Identity(res.id));
                    }).unwrap();
                    return Ok(());
                },
            });
        }
    }));
}
