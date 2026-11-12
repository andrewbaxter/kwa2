use {
    crate::{
        api::req_post_json,
        localdata::{
            get_or_req_api_channelmember,
            req_api_channelmembers,
        },
        pageutil::build_nol_form,
        state::{
            Ministate,
            MinistateChannelMember,
            goto_replace_ministate,
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
    #[title("Can post")]
    can_post: bool,
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelMember) -> El {
    return build_nol_form(
        //. .
        pc,
        &Ministate::ChannelMember(s.clone()),
        "Edit member",
        get_or_req_api_channelmember(&s.channel, &s.identity).map({
            let eg = pc.eg();
            let s = s.clone();
            move |member| {
                let (form_els, form_state) = Form_::new_form("", Some(&Form_ { can_post: member.res.can_post }));
                let form_state = Rc::new(form_state);
                return (form_els.error, form_els.elements, async move |_idem| {
                    let Ok(new_values) = form_state.parse() else {
                        return Ok(());
                    };
                    if new_values.can_post != member.res.can_post {
                        req_post_json(&state().env.base_url, c2s::ChannelMemberModify {
                            channel: s.channel.clone(),
                            member: member.res.identity.clone(),
                            can_post: Some(new_values.can_post),
                        }).await?;
                        req_api_channelmembers(&s.channel, None).await?;
                    }
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &Ministate::ChannelMember(s.clone()));
                    }).unwrap();
                    return Ok(());
                });
            }
        }),
    );
}
