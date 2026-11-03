use {
    crate::{
        api::req_post_json,
        pageutil::{
            build_nol_form,
            req_form_portrait,
            FormPortrait,
        },
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
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::c2s::{
        self,
        ModifyOption,
    },
    spaghettinuum::interface::identity::Identity,
    std::rc::Rc,
};

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Name")]
    name: String,
    #[title("Description")]
    description: String,
    #[title("Shown name")]
    shown_name: Option<String>,
    #[title("Shown description")]
    shown_description: Option<String>,
    #[title("Portrait")]
    portrait: FormPortrait,
}

pub fn build(pc: &mut ProcessingContext, id: &Identity) -> El {
    return build_nol_form(
        //. .
        pc,
        &Ministate::Identity(id.clone()),
        "Edit identity",
        get_or_req_api_identity(id, true).map_async({
            let eg = pc.eg();
            async move |local| {
                let portrait = req_form_portrait(&local.res.portrait).await?;
                let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                    name: local.res.name.clone(),
                    description: local.res.description.clone(),
                    shown_name: local.res.shown_name.clone(),
                    shown_description: local.res.shown_description.clone(),
                    portrait: portrait,
                }));
                let form_state = Rc::new(form_state);
                return Ok((form_els.error, form_els.elements, async move |_idem| {
                    let Ok(new_values) = form_state.parse() else {
                        return Ok(());
                    };
                    let res = req_post_json(&state().env.base_url, c2s::IdentityModify {
                        id: local.res.id.clone(),
                        name: if new_values.name == local.res.name {
                            None
                        } else {
                            Some(new_values.name)
                        },
                        description: if new_values.description == local.res.description {
                            None
                        } else {
                            Some(new_values.description)
                        },
                        shown_name: if new_values.shown_name == local.res.shown_name {
                            None
                        } else {
                            Some(ModifyOption { value: new_values.shown_name })
                        },
                        shown_description: if new_values.shown_description == local.res.shown_description {
                            None
                        } else {
                            Some(ModifyOption { value: new_values.shown_description })
                        },
                        portrait: new_values.portrait.into_modify().await?,
                    }).await?;
                    localdata::ensure_identity(res.clone()).await;
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::Identity(res.id));
                    }).unwrap();
                    return Ok(());
                }));
            }
        }),
    );
}
