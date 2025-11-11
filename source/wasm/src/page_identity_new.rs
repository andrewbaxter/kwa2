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

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Short memo")]
    memo_short: String,
    #[title("Extra memo")]
    memo_long: String,
}

pub fn build(pc: &mut ProcessingContext) -> El {
    let eg = pc.eg();
    let (form_els, form_state) = Form_::new_form("", None);
    let form_state = Rc::new(form_state);
    return build_form(
        format!("New identity"),
        Ministate::Identities,
        form_els.error.unwrap(),
        form_els.elements,
        async move |idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            let res = req_post_json(&state().env.base_url, c2s::IdentityCreate {
                idem: Some(idem.to_string()),
                memo_short: new_values.memo_short,
                memo_long: new_values.memo_long,
            }).await?;
            localdata::ensure_identity(res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Identity(res.id));
            }).unwrap();
            return Ok(());
        },
    );
}
