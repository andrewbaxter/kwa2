use {
    crate::{
        api::{
            channel_owner_file_url,
            req_file_blob,
            req_get,
            req_post_json,
        },
        async_::bg_val,
        js::style_export,
        localdata::{
            self,
            get_or_req_api_contact_opt,
            NowOrLater,
        },
        pageutil::{
            build_nol_edit_page,
            read_portrait,
            req_form_portrait,
            EditPage,
            FormPortrait,
        },
        state::{
            build_ministate,
            ministate_octothorpe,
            state,
            Ministate,
            MinistateContact,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    rooting_forms::{
        impl_file::FileValue,
        Form,
    },
    shared::interface::{
        shared::Portrait,
        wire::c2s::{
            self,
            ModifyOption,
        },
    },
    spaghettinuum::byteszb32::BytesZb32,
    std::rc::Rc,
};

//# genemichaels-norm-skip
#[derive(rooting_forms::Form)]
struct Form_ {
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
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateContact) -> El {
    let local = get_or_req_api_contact_opt(&s.identity, true);
    let page = NowOrLater::Later(bg_val({
        let eg = pc.eg();
        let s = s.clone();
        async move {
            let local = match local {
                NowOrLater::Now(v) => v,
                NowOrLater::Later(v) => v.get().await?.flatten(),
            }.map(|l| l.res);
            let title;
            let initial;
            match &local {
                Some(local) => {
                    title = local.name.clone();
                    initial = Form_ {
                        name: local.name.clone(),
                        description: local.description.clone(),
                        shown_name: local.shown_name.clone(),
                        shown_description: local.shown_description.clone(),
                        portrait: req_form_portrait(&local.portrait).await?,
                    };
                },
                None => {
                    let remote = match &s.channel {
                        Some(channel) => req_get(c2s::GetChannelMemberInfo {
                            channel: channel.clone(),
                            member: s.identity.clone(),
                        }).await?,
                        None => None,
                    };
                    let remote = remote.unwrap_or(c2s::MemberInfoRes {
                        name: None,
                        description: None,
                        portrait: None,
                    });
                    let portrait = match (&remote.portrait, &s.channel) {
                        (Some(p), Some(channel)) => Some(
                            req_file_blob(&channel_owner_file_url(channel, p), &p.mime_type).await?,
                        ),
                        _ => None,
                    };
                    title = remote.name.clone().unwrap_or_default();
                    initial = Form_ {
                        name: remote.name.unwrap_or_default(),
                        description: remote.description.unwrap_or_default(),
                        shown_name: None,
                        shown_description: None,
                        portrait: FormPortrait(FileValue::Keep(portrait)),
                    };
                },
            }
            let (form_els, form_state) = Form_::new_form("", Some(&initial));
            let form_state = Rc::new(form_state);
            let mut menu = vec![style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Channels"),
                link: ministate_octothorpe(&Ministate::ContactChannels(s.clone())),
                image: None,
                generated_image: None,
            }).root, style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Posts"),
                link: ministate_octothorpe(&Ministate::ContactPosts(s.clone())),
                image: None,
                generated_image: None,
            }).root];
            if local.is_some() {
                menu.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ContactDelete(s.clone())),
                    image: None,
                    generated_image: None,
                }).root);
            }
            return Ok(Some(EditPage {
                center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                    text: if title.is_empty() {
                        s.identity.to_string()
                    } else {
                        title
                    },
                    link: None,
                }).root,
                submit_text: if local.is_some() {
                    format!("Save")
                } else {
                    format!("Save as contact")
                },
                form_error: form_els.error,
                form: form_els.elements,
                menu: menu,
                send: async move |_idem| {
                    let Ok(new_values) = form_state.parse() else {
                        return Ok(());
                    };
                    let req = match &local {
                        Some(local) => c2s::ContactModify {
                            id: s.identity.clone(),
                            name: if new_values.name == local.name {
                                None
                            } else {
                                Some(new_values.name)
                            },
                            description: if new_values.description == local.description {
                                None
                            } else {
                                Some(new_values.description)
                            },
                            shown_name: if new_values.shown_name == local.shown_name {
                                None
                            } else {
                                Some(ModifyOption { value: new_values.shown_name })
                            },
                            shown_description: if new_values.shown_description == local.shown_description {
                                None
                            } else {
                                Some(ModifyOption { value: new_values.shown_description })
                            },
                            portrait: new_values.portrait.into_modify().await?,
                        },
                        None => c2s::ContactModify {
                            id: s.identity.clone(),
                            name: Some(new_values.name),
                            description: Some(new_values.description),
                            shown_name: Some(ModifyOption { value: new_values.shown_name }),
                            shown_description: Some(ModifyOption { value: new_values.shown_description }),
                            portrait: match new_values.portrait.0 {
                                FileValue::Keep(Some(blob)) => {
                                    let mime_type = blob.type_();
                                    let data =
                                        gloo::file::futures::read_as_bytes(&gloo::file::Blob::from(blob))
                                            .await
                                            .map_err(|e| e.to_string())?;
                                    Some(ModifyOption { value: Some(Portrait {
                                        mime_type: mime_type,
                                        data: BytesZb32(data),
                                    }) })
                                },
                                FileValue::Keep(None) | FileValue::Clear => None,
                                FileValue::Set(f) => Some(ModifyOption { value: Some(read_portrait(f).await?) }),
                            },
                        },
                    };
                    let res = req_post_json(&state().env.base_url, req).await?;
                    localdata::ensure_contact(res).await;
                    eg.event(|pc| {
                        build_ministate(pc, &Ministate::Contact(s.clone()));
                    }).unwrap();
                    return Ok(());
                },
            }));
        }
    }));
    return build_nol_edit_page(pc, &s.back, page);
}
