use {
    crate::{
        api::req_post_json,
        localdata::{
            self,
        },
        pageutil::build_form,
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
    shared::interface::wire::c2s::{
        self,
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

pub fn build(pc: &mut ProcessingContext, identity: &Identity) -> El {
    let eg = pc.eg();
    let (form_els, form_state) = Form_::new_form("", None);
    let form_state = Rc::new(form_state);
    return build_form(
        //. .
        pc,
        format!("New invite"),
        Ministate::IdentityInvites(identity.clone()),
        form_els.error,
        form_els.elements,
        {
            let identity = identity.clone();
            async move |idem| {
                let Ok(new_values) = form_state.parse() else {
                    return Ok(());
                };
                let res = req_post_json(&state().env.base_url, c2s::IdentityInviteCreate {
                    identity: identity.clone(),
                    idem: Some(idem.to_string()),
                    name: new_values.name,
                    description: new_values.description,
                    shown_name: new_values.shown_name,
                    shown_description: new_values.shown_description,
                    single_use: new_values.single_use,
                    expiry: new_values.expiry,
                }).await?;
                localdata::ensure_identityinvite(res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::IdentityInvite(MinistateIdentityInvite {
                        identity: identity.clone(),
                        invite: res.id,
                    }));
                }).unwrap();
                return Ok(());
            }
        },
    );
}
