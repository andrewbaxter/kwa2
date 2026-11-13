use {
    crate::{
        api::req_post_json,
        localdata::{
            self,
        },
        pageutil::{
            FormLevel,
            build_form,
        },
        state::{
            Ministate,
            MinistateChannelInvite,
            MinistateChannelInviteNew,
            MinistateChannelInviteNewFrom,
            goto_replace_ministate,
            state,
        },
    },
    jiff::Timestamp,
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        shared::Level,
        wire::c2s::{
            self,
        },
    },
    std::rc::Rc,
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
    #[title("Single use")]
    single_use: bool,
    #[title("Expiry")]
    expiry: Option<Timestamp>,
    #[title("Joiner level")]
    level: FormLevel,
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelInviteNew) -> El {
    let channel = &s.channel;
    let eg = pc.eg();
    let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
        name: String::new(),
        description: String::new(),
        shown_name: None,
        shown_description: None,
        single_use: false,
        expiry: None,
        level: FormLevel(Level::Writer),
    }));
    let form_state = Rc::new(form_state);
    return build_form(
        //. .
        pc,
        format!("New invite"),
        match s.from {
            MinistateChannelInviteNewFrom::Invites => Ministate::ChannelInvites(channel.clone()),
            MinistateChannelInviteNewFrom::Members => Ministate::ChannelMembers(channel.clone()),
        },
        form_els.error,
        form_els.elements,
        {
            let channel = channel.clone();
            async move |idem| {
                let Ok(new_values) = form_state.parse() else {
                    return Ok(());
                };
                let res = req_post_json(&state().env.base_url, c2s::ChannelInviteCreate {
                    channel: channel.clone(),
                    idem: Some(idem.to_string()),
                    name: new_values.name,
                    description: new_values.description,
                    shown_name: new_values.shown_name,
                    shown_description: new_values.shown_description,
                    single_use: new_values.single_use,
                    expiry: new_values.expiry,
                    level: new_values.level.0,
                }).await?;
                localdata::ensure_channelinvite(res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelInvite(MinistateChannelInvite {
                        channel: channel.clone(),
                        invite: res.id,
                    }));
                }).unwrap();
                return Ok(());
            }
        },
    );
}
