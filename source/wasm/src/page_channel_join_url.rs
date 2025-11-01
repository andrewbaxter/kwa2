use {
    crate::{
        api::req_post_json,
        formutil::{
            build_form,
            FormIdentity,
        },
        localdata,
        state::{
            goto_replace_ministate,
            state,
            Ministate,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::wire::{
        c2s,
        kwaurl::{
            KwaUrl,
            KwaUrlChannelInvite,
            KwaUrlIdentityInvite,
        },
    },
    std::{
        rc::Rc,
        str::FromStr,
    },
};

enum FormKwaUrlInvite {
    Identity(KwaUrlIdentityInvite),
    Channel(KwaUrlChannelInvite),
}

impl FromStr for FormKwaUrlInvite {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let u = KwaUrl::from_string(s)?;
        match u {
            KwaUrl::Channel(_) => {
                return Err(format!("This is a URL to a channel, not an invitation"));
            },
            KwaUrl::Message(_) => {
                return Err(format!("This is a URL to a message, not an invitation"));
            },
            KwaUrl::ChannelInvite(c) => {
                return Ok(FormKwaUrlInvite::Channel(c));
            },
            KwaUrl::IdentityInvite(c) => {
                return Ok(FormKwaUrlInvite::Identity(c));
            },
        }
    }
}

impl ToString for FormKwaUrlInvite {
    fn to_string(&self) -> String {
        match self {
            FormKwaUrlInvite::Identity(u) => KwaUrl::IdentityInvite(u.clone()).to_string(),
            FormKwaUrlInvite::Channel(u) => KwaUrl::ChannelInvite(u.clone()).to_string(),
        }
    }
}

impl<C: 'static + Clone> rooting_forms::FormWith<C> for FormKwaUrlInvite {
    fn new_form_with_(
        _context: &C,
        field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (rooting_forms::FormElements, Box<dyn rooting_forms::FormState<Self>>) {
        return rooting_forms::impl_str::FromStrFormState::new::<String, FormKwaUrlInvite>(
            field,
            &from.map(|x| x.to_string()).unwrap_or_default(),
        );
    }
}

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("Join as")]
    identity: FormIdentity,
    #[title("kwa: invitation URL")]
    url: FormKwaUrlInvite,
}

pub fn build(pc: &mut ProcessingContext) -> El {
    let eg = pc.eg();
    let (form_els, form_state) = Form_::new_form("", None);
    let form_state = Rc::new(form_state);
    return build_form(
        format!("Join by URL"),
        Ministate::TopAdd,
        form_els.error.unwrap(),
        form_els.elements,
        async move |_idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            let res;
            match new_values.url {
                FormKwaUrlInvite::Identity(u) => {
                    res = req_post_json(&state().env.base_url, c2s::ChannelJoinIdentity {
                        identity: u.identity,
                        code: u.code,
                    }).await?;
                },
                FormKwaUrlInvite::Channel(u) => {
                    res = req_post_json(&state().env.base_url, c2s::ChannelJoinChannel {
                        channel: u.channel,
                        code: u.code,
                    }).await?;
                },
            }
            localdata::ensure_channel(res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Channel(res.id));
            }).unwrap();
            return Ok(());
        },
    );
}
