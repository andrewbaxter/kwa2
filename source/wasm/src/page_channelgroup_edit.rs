use {
    crate::{
        api::req_post_json,
        pageutil::build_nol_form,
        state::{
            Ministate,
            MinistateChannelGroup,
            get_or_req_channelgroup,
            goto_replace_ministate,
            pull_top,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        shared::ChannelGroupId,
        wire::c2s::{
            self,
        },
    },
    std::rc::Rc,
};

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Name")]
    name: String,
    #[title("Description")]
    description: String,
}

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId) -> El {
    return build_nol_form(
        //. .
        pc,
        &Ministate::ChannelGroupMenu(id.clone()),
        "Edit group",
        get_or_req_channelgroup(&pc.eg(), id, false).map({
            let eg = pc.eg();
            move |value| {
                let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                    name: value.name.get(),
                    description: value.description.get(),
                }));
                let form_state = Rc::new(form_state);
                return (form_els.error, form_els.elements, async move |_idem| {
                    let Ok(new_values) = form_state.parse() else {
                        return Ok(());
                    };
                    let res = req_post_json(&state().env.base_url, c2s::ChannelGroupModify {
                        id: value.id.clone(),
                        name: if new_values.name == *value.name.borrow() {
                            None
                        } else {
                            Some(new_values.name)
                        },
                        description: if new_values.description == *value.description.borrow() {
                            None
                        } else {
                            Some(new_values.description)
                        },
                    }).await?;
                    pull_top(&eg).await;
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::ChannelGroup(MinistateChannelGroup {
                            id: res.id,
                            reset_id: None,
                        }));
                    }).unwrap();
                    return Ok(());
                });
            }
        }),
    );
}
