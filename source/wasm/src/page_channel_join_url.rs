use {
    crate::{
        js::style_export,
        pageutil::build_form,
        state::{
            ChannelJoinSource,
            Ministate,
            MinistateChannelJoinConfirm,
            goto_replace_ministate,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::Form,
    shared::interface::{
        shared::QualifiedChannelId,
        wire::kwaurl::{
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

pub enum FormKwaUrlInvite {
    Identity(KwaUrlIdentityInvite),
    Channel(KwaUrlChannelInvite),
    PublicChannel(QualifiedChannelId),
}

impl FormKwaUrlInvite {
    pub fn into_url(self) -> KwaUrl {
        match self {
            FormKwaUrlInvite::Identity(u) => return KwaUrl::IdentityInvite(u),
            FormKwaUrlInvite::Channel(u) => return KwaUrl::ChannelInvite(u),
            FormKwaUrlInvite::PublicChannel(u) => return KwaUrl::Channel(u),
        }
    }
}

impl FromStr for FormKwaUrlInvite {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let u = KwaUrl::from_string(s)?;
        match u {
            KwaUrl::Channel(c) => {
                return Ok(FormKwaUrlInvite::PublicChannel(c));
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
            FormKwaUrlInvite::Identity(u) => return KwaUrl::IdentityInvite(u.clone()).to_string(),
            FormKwaUrlInvite::Channel(u) => return KwaUrl::ChannelInvite(u.clone()).to_string(),
            FormKwaUrlInvite::PublicChannel(u) => return KwaUrl::Channel(u.clone()).to_string(),
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
        let (els, state) =
            rooting_forms::impl_str::FromStrFormState::new::<String, FormKwaUrlInvite>(
                field,
                &from.map(|x| x.to_string()).unwrap_or_default(),
            );
        els.elements[0].ref_classes(&[&style_export::class_code().value]);
        return (els, state);
    }
}

#[derive(rooting_forms::Form)]
struct Form_ {
    #[title("URL")]
    url: FormKwaUrlInvite,
}

pub fn build(pc: &mut ProcessingContext) -> El {
    let eg = pc.eg();
    let (form_els, form_state) = Form_::new_form("", None);
    let form_state = Rc::new(form_state);
    return build_form(
        //. .
        pc,
        format!("Join by URL"),
        Ministate::TopAdd,
        form_els.error,
        form_els.elements,
        async move |_idem| {
            let Ok(new_values) = form_state.parse() else {
                return Ok(());
            };
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::ChannelJoinConfirm(MinistateChannelJoinConfirm {
                    url: new_values.url.into_url(),
                    source: ChannelJoinSource::Url,
                }));
            }).unwrap();
            return Ok(());
        },
    );
}
