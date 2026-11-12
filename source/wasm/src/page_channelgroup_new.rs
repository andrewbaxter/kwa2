use {
    crate::{
        api::req_post_json,
        pageutil::build_form,
        state::{
            Ministate,
            goto_replace_ministate,
            pull_top,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::c2s,
    std::rc::Rc,
};

//# genemichaels-norm-skip
#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Name")]
    name: String,
    #[title("Description")]
    description: String,
}

pub fn build(pc: &mut ProcessingContext) -> El {
    let eg = pc.eg();
    let (form_els, form_state) = Form_::new_form("", None);
    let form_state = Rc::new(form_state);
    return build_form(
        //. .
        pc,
        format!("New group"),
        Ministate::TopAdd,
        form_els.error,
        form_els.elements,
        async move |idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            req_post_json(&state().env.base_url, c2s::ChannelGroupCreate {
                idem: Some(idem.to_string()),
                name: new_values.name,
                description: new_values.description,
            }).await?;
            pull_top(&eg).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Top);
            }).unwrap();
            return Ok(());
        },
    );
}
