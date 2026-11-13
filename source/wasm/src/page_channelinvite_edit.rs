use {
    crate::{
        api::req_post_json,
        localdata::{
            self,
            get_or_req_api_channelinvite,
        },
        pageutil::{
            FormLevel,
            build_nol_form,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
            MinistateChannelInvite,
        },
    },
    jiff::Timestamp,
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        shared::{
            ChannelInviteId,
            QualifiedChannelId,
        },
        wire::c2s::{
            self,
            ModifyOption,
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

pub fn build(pc: &mut ProcessingContext, channel: &QualifiedChannelId, id: &ChannelInviteId) -> El {
    return build_nol_form(pc, &Ministate::ChannelInvite(MinistateChannelInvite {
        channel: channel.clone(),
        invite: id.clone(),
    }), "Edit invite", get_or_req_api_channelinvite(channel, id, true).map({
        let eg = pc.eg();
        let channel = channel.clone();
        move |local| {
            let (form_els, form_state) = Form_::new_form("", Some(&Form_ {
                name: local.res.name.clone(),
                description: local.res.description.clone(),
                shown_name: local.res.shown_name.clone(),
                shown_description: local.res.shown_description.clone(),
                single_use: local.res.single_use.clone(),
                expiry: local.res.expiry.clone(),
                level: FormLevel(local.res.level),
            }));
            let form_state = Rc::new(form_state);
            return (form_els.error, form_els.elements, async move |_idem| {
                let Ok(new_values) = form_state.parse() else {
                    return Ok(());
                };
                let res = req_post_json(&state().env.base_url, c2s::ChannelInviteModify {
                    channel: channel.clone(),
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
                    level: if new_values.level.0 == local.res.level {
                        None
                    } else {
                        Some(new_values.level.0)
                    },
                }).await?;
                localdata::ensure_channelinvite(res.clone()).await;
                eg.event(|pc| {
                    goto_replace_ministate(pc, &state().log, &Ministate::ChannelInvite(MinistateChannelInvite {
                        channel: channel.clone(),
                        invite: res.id,
                    }));
                }).unwrap();
                return Ok(());
            });
        }
    }));
}
