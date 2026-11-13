use {
    crate::{
        api::identity_image,
        blocks::{
            ViewBlock,
            format_size,
            order_blocks,
        },
        event_editor::event_timeline_text,
        media_viewer::open_media_viewer,
        chat::{
            ChatMode,
            ChatModeEditMessage,
            ChatModeReplyMessage,
            ChatModeReplyMessageTarget,
            ChatModeReplyMessageTargetOutbox,
        },
        chat_entry::{
            ChatEntryMessage,
            ChatEntryMessageInternal,
        },
        js::style_export,
        localdata,
        state::{
            CurrentChatSource,
            Ministate,
            MinistateChannel,
            MinistateChannelGroup,
            MinistateChannelGroupResetId,
            MinistateContact,
            MinistateMessageHistory,
            goto_replace_ministate,
            identity_portrait,
            ministate_octothorpe,
            state,
        },
    },
    lunk::{
        ProcessingContext,
        link,
    },
    by_address::ByAddress,
    flowcontrol::shed,
    rooting::El,
    shared::interface::shared::ModerationState,
    std::rc::Rc,
};

pub fn build_chat_entry_message(pc: &mut ProcessingContext, m: &ChatEntryMessage) -> El {
    let out = style_export::cont_chat_entry().root;
    let left = !localdata::get_stored_api_identities(None).into_iter().any(|x| x.res.id == m.sender);
    out.ref_own(
        |root| link!(
            (pc = pc),
            (int = m.internal.clone(), hidden = m.hidden.clone()),
            (),
            (
                root = root.weak(),
                left = left,
                sender = m.sender.clone(),
                chat_state = m.chat_state.clone(),
                message_id = m.message_id.clone(),
                client_key = m.client_key.clone()
            ) {
                let root = root.upgrade()?;
                root.ref_clear();
                if *hidden.borrow() {
                    return None;
                }
                match &*int.borrow() {
                    ChatEntryMessageInternal::Deleted => {
                        root.ref_push(
                            style_export::cont_chat_entry_mode_deleted(
                                style_export::ContChatEntryModeDeletedArgs { left: *left }
                            ).root
                        );
                    },
                    ChatEntryMessageInternal::Message(m) => {
                        let m_el =
                            style_export::cont_chat_entry_mode_message(style_export::ContChatEntryModeMessageArgs {
                                left: *left,
                                date: m.time.to_string(),
                            });
                        let portrait = identity_portrait(pc, &sender);
                        m_el
                            .portrait
                            .ref_own(
                                |portrait_el| link!(
                                    (_pc = pc),
                                    (portrait = portrait),
                                    (),
                                    (portrait_el = portrait_el.weak(), sender = sender.clone()) {
                                        let portrait_el = portrait_el.upgrade()?;
                                        let img = identity_image(&sender, &*portrait.borrow());
                                        portrait_el.ref_clear();
                                        portrait_el.ref_push(
                                            style_export::leaf_chat_entry_portrait(
                                                style_export::LeafChatEntryPortraitArgs {
                                                    image: img.image,
                                                    generated_image: img.generated_image,
                                                },
                                            ).root,
                                        );
                                    }
                                ),
                            );
                        m_el.portrait.ref_on("click", {
                            let eg = pc.eg();
                            let sender = sender.clone();
                            let left = *left;
                            let channel = match (&message_id, &client_key) {
                                (Some(message_id), _) => message_id.channel.clone(),
                                (None, Some((channel, _, _))) => channel.clone(),
                                (None, None) => unreachable!(),
                            };
                            move |ev| {
                                ev.stop_propagation();
                                let target = if left {
                                    Ministate::Contact(MinistateContact {
                                        identity: sender.clone(),
                                        channel: Some(channel.clone()),
                                        back: Box::new(state().ministate.borrow().clone()),
                                    })
                                } else {
                                    Ministate::Identity(sender.clone())
                                };
                                eg.event(|pc| {
                                    goto_replace_ministate(pc, &state().log, &target);
                                }).unwrap();
                            }
                        });
                        if let Some(moderation) =
                            message_id.as_ref().and_then(|id| chat_state.moderation_states.borrow().get(id).cloned()) {
                            let label =
                                style_export::leaf_chat_entry_moderation_state(
                                    style_export::LeafChatEntryModerationStateArgs { text: String::new() },
                                ).root;
                            label.ref_own(
                                |label| link!((_pc = pc), (moderation = moderation), (), (label = label.weak()) {
                                    let label = label.upgrade()?;
                                    label.ref_text(match *moderation.borrow() {
                                        ModerationState::Pending => "Pending",
                                        ModerationState::Approved => "Approved",
                                        ModerationState::Rejected => "Rejected",
                                    });
                                })
                            );
                            m_el.actions.ref_push(label);
                        }
                        shed!{
                            let Some(message_id) = message_id else {
                                break;
                            };
                            let Some(own_identity) =
                                chat_state
                                    .channels_meta
                                    .borrow()
                                    .iter()
                                    .find(|c| c.2 == message_id.channel)
                                    .map(|c| c.1.clone()) else {
                                    break;
                                };
                            m_el.actions.ref_push(style_export::leaf_chat_entry_message_button_edit().root.on("click", {
                                let eg = pc.eg();
                                let chat_state = Rc::downgrade(chat_state);
                                let message_id = message_id.clone();
                                let own_identity = own_identity.clone();
                                let blocks = m.blocks.clone();
                                move |ev| {
                                    ev.stop_propagation();
                                    let Some(chat_state) = chat_state.upgrade() else {
                                        return;
                                    };
                                    eg.event(|pc| {
                                        chat_state.mode.set(pc, ChatMode::EditMessage(ChatModeEditMessage {
                                            target: message_id.clone(),
                                            own_identity: own_identity.clone(),
                                            blocks: ByAddress(Rc::new(blocks.borrow().clone())),
                                        }));
                                    }).unwrap();
                                }
                            }));
                            let back = match &chat_state.source {
                                CurrentChatSource::Channel(id) => Ministate::Channel(MinistateChannel {
                                    id: id.clone(),
                                    own_identity: own_identity.clone(),
                                    reset_id: Some(message_id.clone()),
                                }),
                                CurrentChatSource::Group(id) => Ministate::ChannelGroup(MinistateChannelGroup {
                                    id: id.clone(),
                                    reset_id: Some(MinistateChannelGroupResetId {
                                        own_identity: own_identity.clone(),
                                        message: message_id.clone(),
                                    }),
                                }),
                            };
                            m_el
                                .actions
                                .ref_push(
                                    style_export::leaf_chat_entry_message_button_history(
                                        style_export::LeafChatEntryMessageButtonHistoryArgs {
                                            link: ministate_octothorpe(
                                                &Ministate::MessageHistory(MinistateMessageHistory {
                                                    id: message_id.clone(),
                                                    back: Box::new(back),
                                                })
                                            ),
                                        },
                                    ).root,
                                );
                        }
                        m_el
                            .root
                            .ref_own(
                                |msg_root| link!(
                                    (_pc = pc),
                                    (blocks = m.blocks.clone()),
                                    (),
                                    (msg_root = msg_root.weak(), body_el = m_el.body.clone()) {
                                        let msg_root = msg_root.upgrade()?;
                                        msg_root.ref_modify_classes(
                                            &[(&style_export::class_state_hidden().value, blocks.borrow().is_empty())],
                                        );
                                        build_message_body(&body_el, &*blocks.borrow());
                                    }
                                ),
                            );
                        m_el
                            .root
                            .ref_own(
                                |el_| link!(
                                    (_pc = pc),
                                    (mode = chat_state.mode.clone()),
                                    (),
                                    (
                                        el_ = el_.weak(),
                                        message_id = message_id.clone(),
                                        client_key = client_key.clone()
                                    ) {
                                        let el_ = el_.upgrade()?;
                                        let selected = shed!{
                                            let Some(mode) = mode.borrow().focus() else {
                                                break false;
                                            };
                                            match &mode.target {
                                                ChatModeReplyMessageTarget::Channel(target) => {
                                                    break message_id.as_ref() == Some(target);
                                                },
                                                ChatModeReplyMessageTarget::Outbox(target) => {
                                                    let Some((channel, identity, client_id)) = client_key else {
                                                        break false;
                                                    };
                                                    break *channel == target.channel &&
                                                        *identity == mode.own_identity &&
                                                        *client_id == target.message;
                                                },
                                            }
                                        };
                                        el_.ref_modify_classes(
                                            &[(&style_export::class_state_selected().value, selected)],
                                        );
                                    }
                                ),
                            );
                        m_el.root.ref_on("click", {
                            let eg = pc.eg();
                            let chat_state = Rc::downgrade(chat_state);
                            let message_id = message_id.clone();
                            let client_key = client_key.clone();
                            move |_| {
                                let Some(chat_state) = chat_state.upgrade() else {
                                    return;
                                };
                                let new_mode = shed!{
                                    match &message_id {
                                        Some(message_id) => {
                                            let Some(own_identity) =
                                                chat_state
                                                    .channels_meta
                                                    .borrow()
                                                    .iter()
                                                    .find(|c| c.2 == message_id.channel)
                                                    .map(|c| c.1.clone()) else {
                                                    state()
                                                        .log
                                                        .log(
                                                            &format!(
                                                                "No local identity for channel [{:?}] of message",
                                                                message_id.channel
                                                            ),
                                                        );
                                                    return;
                                                };
                                            break ChatMode::ReplyMessage(ChatModeReplyMessage {
                                                target: ChatModeReplyMessageTarget::Channel(message_id.clone()),
                                                own_identity: own_identity,
                                            });
                                        },
                                        None => {
                                            let (channel, identity, client_id) = client_key.as_ref().unwrap();
                                            break ChatMode::ReplyMessage(ChatModeReplyMessage {
                                                target: ChatModeReplyMessageTarget::Outbox(
                                                    ChatModeReplyMessageTargetOutbox {
                                                        channel: channel.clone(),
                                                        message: client_id.clone(),
                                                    },
                                                ),
                                                own_identity: identity.clone(),
                                            });
                                        },
                                    }
                                };
                                eg.event(|pc| {
                                    chat_state.mode.set(pc, new_mode);
                                }).unwrap();
                            }
                        });
                        root.ref_push(m_el.root);
                    },
                }
            }
        ),
    );
    return out;
}

pub fn build_message_body(body_el: &El, blocks: &[ViewBlock]) {
    body_el.ref_clear();
    let ordered = order_blocks(blocks);
    for text in &ordered.texts {
        body_el.ref_push(
            style_export::leaf_chat_entry_message_text(
                style_export::LeafChatEntryMessageTextArgs { content: malarkdowney::build_readonly(text) },
            ).root,
        );
    }
    for event in &ordered.events {
        let timeline = match event_timeline_text(event) {
            Ok(t) => t,
            Err(e) => format!("Invalid event time: {}", e),
        };
        body_el.ref_push(
            style_export::leaf_chat_entry_message_event(
                style_export::LeafChatEntryMessageEventArgs { timeline: timeline },
            ).root,
        );
    }
    if !ordered.media.is_empty() {
        let mut squares = vec![];
        for (i, file) in ordered.media.iter().enumerate() {
            let square =
                style_export::leaf_chat_entry_message_media_square(
                    style_export::LeafChatEntryMessageMediaSquareArgs {
                        content: style_export::leaf_media_thumb(style_export::LeafMediaThumbArgs {
                            src: file.url(true),
                            video: file.is_video(),
                            src_is_video: file.thumbnail_is_video(),
                        }).root,
                    },
                ).root;
            square.ref_on("click", {
                let media = ordered.media.clone();
                move |ev| {
                    ev.stop_propagation();
                    open_media_viewer(media.clone(), i);
                }
            });
            squares.push(square);
        }
        body_el.ref_push(
            style_export::cont_chat_entry_message_media(
                style_export::ContChatEntryMessageMediaArgs { children: squares },
            ).root,
        );
    }
    for file in &ordered.files {
        body_el.ref_push(style_export::leaf_chat_entry_message_file(style_export::LeafChatEntryMessageFileArgs {
            name: file.download_name(),
            size: format_size(file.size),
            link: file.url(false),
        }).root.on("click", |ev| ev.stop_propagation()));
    }
    for extension in &ordered.extensions {
        body_el.ref_push(
            style_export::leaf_chat_entry_message_extension(style_export::LeafChatEntryMessageExtensionArgs {
                key: extension.key.clone(),
                json: serde_json::to_string_pretty(&extension.data).unwrap(),
            }).root.on("click", |ev| ev.stop_propagation()),
        );
    }
}
