use {
    crate::{
        api::req_post_json,
        pageutil::{
            FormMemberRetention,
            FormOptChannelGroup,
            FormOwnerRetention,
            FormPortrait,
            build_nol_form,
            req_form_portrait,
        },
        state::{
            Ministate,
            MinistateChannel,
            MinistateChannelSub,
            get_or_req_channel,
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
            ChannelRetention,
        },
    },
    std::rc::Rc,
};

//# genemichaels-norm-skip
#[derive(rooting_forms::Form)]
struct MemberForm_ {
    #[title("Portrait")]
    portrait: FormPortrait,
    #[title("Name")]
    name: String,
    #[title("Description")]
    description: String,
    #[title("Group")]
    group: FormOptChannelGroup,
    #[title("Keep history")]
    retention: FormMemberRetention,
    #[title("Event reminders")]
    reminders: bool,
    #[title("Remind minutes before")]
    reminder_lead_minutes: u64,
}

//# genemichaels-norm-skip
#[derive(rooting_forms::Form)]
struct OwnerForm_ {
    #[title("Portrait")]
    portrait: FormPortrait,
    #[title("Name")]
    name: String,
    #[title("Shown name")]
    shown_name: Option<String>,
    #[title("Description")]
    description: String,
    #[title("Shown description")]
    shown_description: Option<String>,
    #[title("Group")]
    group: FormOptChannelGroup,
    #[title("Keep history")]
    retention: FormOwnerRetention,
    #[title("Public: anyone can read")]
    public_read: bool,
    #[title("Event reminders")]
    reminders: bool,
    #[title("Remind minutes before")]
    reminder_lead_minutes: u64,
}

struct Values {
    name: String,
    description: String,
    shown: Option<(Option<String>, Option<String>)>,
    portrait: FormPortrait,
    group: Option<ChannelGroupId>,
    retention: ChannelRetention,
    public_read: Option<bool>,
    reminders: bool,
    reminder_lead_minutes: u64,
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelSub) -> El {
    return build_nol_form(
        //. .
        pc,
        &Ministate::ChannelMenu(s.clone()),
        "Edit channel",
        get_or_req_channel(&pc.eg(), &s.id, false).map_async({
            let eg = pc.eg();
            let own_identity = s.own_identity.clone();
            async move |local| {
                let portrait = req_form_portrait(&local.portrait.get()).await?;
                let is_owner = local.id.identity == local.own_identity;
                let (err_el, elements, parse): (Option<El>, Vec<El>, Rc<dyn Fn() -> Result<Values, ()>>) =
                    if is_owner {
                        let (form_els, form_state) = OwnerForm_::new_form("", Some(&OwnerForm_ {
                            name: local.name.get(),
                            description: local.description.get(),
                            shown_name: local.shown_name.get(),
                            shown_description: local.shown_description.get(),
                            portrait: portrait.clone(),
                            group: FormOptChannelGroup(local.group.get()),
                            retention: FormOwnerRetention(local.retention.get()),
                            public_read: local.public_read.get(),
                            reminders: local.reminders.get(),
                            reminder_lead_minutes: local.reminder_lead_minutes.get(),
                        }));
                        (form_els.error, form_els.elements, Rc::new(move || {
                            let v = form_state.parse()?;
                            return Ok(Values {
                                name: v.name,
                                description: v.description,
                                shown: Some((v.shown_name, v.shown_description)),
                                portrait: v.portrait,
                                group: v.group.0,
                                retention: v.retention.0,
                                public_read: Some(v.public_read),
                                reminders: v.reminders,
                                reminder_lead_minutes: v.reminder_lead_minutes,
                            });
                        }))
                    } else {
                        let (form_els, form_state) = MemberForm_::new_form("", Some(&MemberForm_ {
                            name: local.name.get(),
                            description: local.description.get(),
                            portrait: portrait.clone(),
                            group: FormOptChannelGroup(local.group.get()),
                            retention: FormMemberRetention(local.retention.get()),
                            reminders: local.reminders.get(),
                            reminder_lead_minutes: local.reminder_lead_minutes.get(),
                        }));
                        (form_els.error, form_els.elements, Rc::new(move || {
                            let v = form_state.parse()?;
                            return Ok(Values {
                                name: v.name,
                                description: v.description,
                                shown: None,
                                portrait: v.portrait,
                                group: v.group.0,
                                retention: v.retention.0,
                                public_read: None,
                                reminders: v.reminders,
                                reminder_lead_minutes: v.reminder_lead_minutes,
                            });
                        }))
                    };
                return Ok((err_el, elements, async move |_idem| {
                    let Ok(new_values) = parse() else {
                        return Ok(());
                    };
                    let (shown_name, shown_description) = match new_values.shown {
                        Some((shown_name, shown_description)) => (if shown_name == *local.shown_name.borrow() {
                            None
                        } else {
                            Some(c2s::ModifyOption { value: shown_name })
                        }, if shown_description == *local.shown_description.borrow() {
                            None
                        } else {
                            Some(c2s::ModifyOption { value: shown_description })
                        }),
                        None => (None, None),
                    };
                    let res = req_post_json(&state().env.base_url, c2s::ChannelModify {
                        own_identity: own_identity.clone(),
                        id: local.id.clone(),
                        name: if new_values.name == *local.name.borrow() {
                            None
                        } else {
                            Some(new_values.name)
                        },
                        description: if new_values.description == *local.description.borrow() {
                            None
                        } else {
                            Some(new_values.description)
                        },
                        shown_name: shown_name,
                        shown_description: shown_description,
                        group: if new_values.group == *local.group.borrow() {
                            None
                        } else {
                            Some(c2s::ModifyOption { value: new_values.group })
                        },
                        retention: if new_values.retention == local.retention.get() {
                            None
                        } else {
                            Some(new_values.retention)
                        },
                        public_read: match new_values.public_read {
                            Some(read) if read != local.public_read.get() => Some(read),
                            _ => None,
                        },
                        public_post: None,
                        portrait: new_values.portrait.into_modify().await?,
                        reminders: if new_values.reminders == local.reminders.get() {
                            None
                        } else {
                            Some(new_values.reminders)
                        },
                        reminder_lead_minutes: if new_values.reminder_lead_minutes ==
                            local.reminder_lead_minutes.get() {
                            None
                        } else {
                            Some(new_values.reminder_lead_minutes)
                        },
                    }).await?;
                    pull_top(&eg).await;
                    eg.event(|pc| {
                        goto_replace_ministate(pc, &state().log, &&Ministate::Channel(MinistateChannel {
                            id: res.id.clone(),
                            own_identity: own_identity.clone(),
                            reset_id: None,
                        }));
                    }).unwrap();
                    return Ok(());
                }));
            }
        }),
    );
}
