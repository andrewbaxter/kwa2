use {
    crate::{
        api::{
            join_preview_file_url,
            req_file_blob,
            req_post_json,
        },
        async_::bg_val,
        localdata::NowOrLater,
        pageutil::{
            FormIdentity,
            FormPortrait,
            build_nol_form,
            read_portrait,
        },
        state::{
            ChannelJoinSource,
            Ministate,
            MinistateChannelJoinConfirm,
            MinistateChannel,
            goto_replace_ministate,
            pull_top,
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        El,
        el,
    },
    rooting_forms::{
        css::CSS_CLASS_LABEL,
        impl_file::FileValue,
        Form,
        FormWith,
    },
    shared::interface::{
        shared::JoinPortrait,
        wire::{
            c2s,
            kwaurl::KwaUrl,
        },
    },
    std::rc::Rc,
};

#[derive(rooting_forms::Form)]
struct DetailsForm_ {
    #[title("Name")]
    name: String,
    #[title("Description")]
    description: String,
    #[title("Portrait")]
    portrait: FormPortrait,
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelJoinConfirm) -> El {
    let eg = pc.eg();
    let url = s.url.clone();
    let (back_link, title) = match s.source {
        ChannelJoinSource::Url => (Ministate::ChannelJoinUrl, "Join by URL"),
        ChannelJoinSource::Qr => (Ministate::ChannelJoinQr, "Join by QR"),
    };
    return build_nol_form(pc, &back_link, title, NowOrLater::Later(bg_val(async move {
        if let KwaUrl::Message(_) = &url {
            return Err(format!("A message URL can't be joined"));
        }
        let preview = req_post_json(&state().env.base_url, c2s::ChannelJoinPreview { url: url.clone() }).await?;
        let portrait = match &preview.portrait {
            Some(p) => Some(req_file_blob(&join_preview_file_url(&url, p), &p.mime_type).await?),
            None => None,
        };
        let identity_title = "Join as";
        let (identity_els, identity_state) = FormIdentity::new_form_with(&(), identity_title, None);
        let (details_els, details_state) = DetailsForm_::new_form("", Some(&DetailsForm_ {
            name: preview.name,
            description: preview.description,
            portrait: FormPortrait(FileValue::Keep(portrait)),
        }));
        let identity_state = Rc::new(identity_state);
        let details_state = Rc::new(details_state);
        let mut elements = vec![];
        elements.extend(identity_els.error);
        elements.push(el("span").classes(&[CSS_CLASS_LABEL]).text(identity_title));
        elements.extend(identity_els.elements);
        elements.extend(details_els.elements);
        return Ok(Some((details_els.error, elements, async move |_idem| {
            let identity = identity_state.parse();
            let details = details_state.parse();
            let (Ok(identity), Ok(details)) = (identity, details) else {
                return Ok(());
            };
            if details.name.is_empty() {
                return Err(format!("A name is required"));
            }
            let portrait = match details.portrait.0 {
                FileValue::Keep(Some(_)) => Some(JoinPortrait::FromInvitation),
                FileValue::Keep(None) | FileValue::Clear => None,
                FileValue::Set(f) => Some(JoinPortrait::Upload(read_portrait(f).await?)),
            };
            let sender = identity.0;
            let res;
            match url.clone() {
                KwaUrl::IdentityInvite(u) => {
                    res = req_post_json(&state().env.base_url, c2s::ChannelJoinIdentity {
                        identity: u.identity,
                        sender: sender.clone(),
                        code: u.code,
                        name: details.name,
                        description: details.description,
                        portrait: portrait,
                    }).await?;
                },
                KwaUrl::ChannelInvite(u) => {
                    res = req_post_json(&state().env.base_url, c2s::ChannelJoinChannel {
                        channel: u.channel,
                        sender: sender.clone(),
                        code: u.code,
                        name: details.name,
                        description: details.description,
                        portrait: portrait,
                    }).await?;
                },
                KwaUrl::Channel(u) => {
                    res = req_post_json(&state().env.base_url, c2s::ChannelAdd {
                        channel: u,
                        identity: sender.clone(),
                        name: details.name,
                        description: details.description,
                        portrait: portrait,
                    }).await?;
                },
                KwaUrl::Message(_) => {
                    return Err(format!("A message URL can't be joined"));
                },
            }
            pull_top(&eg).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Channel(MinistateChannel {
                    id: res.id.clone(),
                    own_identity: sender,
                    reset_id: None,
                }));
            }).unwrap();
            return Ok(());
        })));
    })));
}
