use {
    crate::{
        api::req_post_json,
        localdata::{
            self,
            greq_api_identityinvites,
        },
        pageutil::build_nol_form,
        state::{
            goto_replace_ministate,
            state,
            Ministate,
            MinistateIdentityInvite,
        },
    },
    jiff::Timestamp,
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        shared::IdentityInviteId,
        wire::c2s::{
            self,
            ModifyOption,
        },
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
    #[title("Single use")]
    single_use: bool,
    #[title("Expiry")]
    expiry: Option<Timestamp>,
}

pub fn build(pc: &mut ProcessingContext, identity: &Identity, id: &IdentityInviteId) -> El {
    return build_nol_form(pc, &Ministate::IdentityInvite(MinistateIdentityInvite {
        identity: identity.clone(),
        invite: id.clone(),
    }), "Edit invite", greq_api_identityinvites(id, true).map({
        let eg = pc.eg();
        let identity = identity.clone();
        let id = id.clone();
        move |local| {
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                name: local.res.name.clone(),
                description: local.res.description.clone(),
                shown_name: local.res.shown_name.clone(),
                shown_description: local.res.shown_description.clone(),
                single_use: local.res.single_use.clone(),
                expiry: local.res.expiry.clone(),
            }));
            let form_state = Rc::new(form_state);
            return (form_els.error, form_els.elements, async move |_idem| {
                let Ok(new_values) = form_state.parse() else {
                    return Ok(());
                };
                let res = req_post_json(&state().env.base_url, c2s::IdentityInviteModify {
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
                    single_use: if new_values.single_use == local.res.single_use {
                        None
                    } else {
                        Some(new_values.single_use)
                    },
                    expiry: if new_values.expiry == local.res.expiry {
                        None
                    } else {
                        Some(ModifyOption { value: new_values.expiry })
                    },
                }).await?;
                localdata::ensure_identityinvite(res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::IdentityInvite(MinistateIdentityInvite {
                        identity: identity.clone(),
                        invite: id.clone(),
                    }));
                }).unwrap();
                return Ok(());
            });
        }
    }));
}
