use {
    crate::{
        api::req_post_json,
        pageutil::build_form,
        localdata::{
            self,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        El,
    },
    rooting_forms::Form,
    shared::interface::wire::c2s::{
        self,
    },
    std::{
        rc::Rc,
    },
};

//# genemichaels-norm-skip
#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Name")]
    name: String,
    #[title("Shown name")]
    shown_name: Option<String>,
    #[title("Description")]
    description: String,
    #[title("Shown description")]
    shown_description: Option<String>,
}

pub fn build(pc: &mut ProcessingContext) -> El {
    let eg = pc.eg();
    let (form_els, form_state) = Form_::new_form("", None);
    let form_state = Rc::new(form_state);
    return build_form(
        //. .
        pc,
        format!("New identity"),
        Ministate::Identities,
        form_els.error,
        form_els.elements,
        async move |idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            let res = req_post_json(&state().env.base_url, c2s::IdentityCreate {
                idem: Some(idem.to_string()),
                name: new_values.name,
                description: new_values.description,
                shown_name: new_values.shown_name,
                shown_description: new_values.shown_description,
            }).await?;
            localdata::ensure_identity(res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Identity(res.id));
            }).unwrap();
            return Ok(());
        },
    );
}
