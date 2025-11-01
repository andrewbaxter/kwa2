use {
    crate::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
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
    #[title("Short memo")]
    memo_short: String,
    #[title("Extra memo")]
    memo_long: String,
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
        format!("New invite"),
        Ministate::Identity(identity.clone()),
        form_els.error.unwrap(),
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
                    memo_short: new_values.memo_short,
                    memo_long: new_values.memo_long,
                    single_use: new_values.single_use,
                    expiry: new_values.expiry,
                }).await?;
                localdata::ensure_identityinvite(res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::IdentityInvite(res.id));
                }).unwrap();
                return Ok(());
            }
        },
    );
}
