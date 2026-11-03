use {
    crate::{
        api::identity_image,
        chat::{
            ChatMode,
            ChatModeReplyMessage,
            ChatModeReplyMessageTarget,
            ChatModeReplyMessageTargetOutbox,
            ChatState,
            ClientIdKey,
        },
        chat_entry::{
            ChatEntryMessage,
            ChatEntryMessageInternal,
        },
        js::style_export,
        localdata,
        state::{
            identity_portrait,
            state,
        },
    },
    lunk::{
        ProcessingContext,
        link,
    },
    rooting::El,
    shared::interface::shared::QualifiedMessageId,
    std::rc::Rc,
};

fn is_selected(mode: &ChatMode, message_id: &Option<QualifiedMessageId>, client_key: &Option<ClientIdKey>) -> bool {
    let ChatMode::ReplyMessage(mode) = mode else {
        return false;
    };
    match &mode.target {
        ChatModeReplyMessageTarget::Channel(target) => {
            return message_id.as_ref() == Some(target);
        },
        ChatModeReplyMessageTarget::Outbox(target) => {
            let Some((channel, identity, client_id)) = client_key else {
                return false;
            };
            return *channel == target.channel && *identity == mode.own_identity && *client_id == target.message;
        },
    }
}

fn reply_mode(
    chat_state: &ChatState,
    message_id: &Option<QualifiedMessageId>,
    client_key: &Option<ClientIdKey>,
) -> Option<ChatMode> {
    match message_id {
        Some(message_id) => {
            let Some(own_identity) =
                chat_state
                    .channels_meta
                    .borrow()
                    .iter()
                    .find(|c| c.2 == message_id.channel)
                    .map(|c| c.1.clone()) else {
                    state().log.log(&format!("No local identity for channel [{:?}] of message", message_id.channel));
                    return None;
                };
            return Some(ChatMode::ReplyMessage(ChatModeReplyMessage {
                target: ChatModeReplyMessageTarget::Channel(message_id.clone()),
                own_identity: own_identity,
            }));
        },
        None => {
            let (channel, identity, client_id) = client_key.as_ref().unwrap();
            return Some(ChatMode::ReplyMessage(ChatModeReplyMessage {
                target: ChatModeReplyMessageTarget::Outbox(ChatModeReplyMessageTargetOutbox {
                    channel: channel.clone(),
                    message: client_id.clone(),
                }),
                own_identity: identity.clone(),
            }));
        },
    }
}

pub fn build_chat_entry_message(pc: &mut ProcessingContext, m: &ChatEntryMessage) -> El {
    let out = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
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
                        m_el
                            .root
                            .ref_own(|_| link!((_pc = pc), (body = m.body.clone()), (), (body_el = m_el.body.clone()) {
                                body_el.ref_text(&*body.borrow());
                            }));
                        m_el
                            .root
                            .ref_own(
                                |el_| link!(
                                    (_pc = pc),
                                    (mode = chat_state.mode.clone()),
                                    (),
                                    (el_ = el_.weak(), message_id = message_id.clone(), client_key = client_key.clone()) {
                                        let el_ = el_.upgrade()?;
                                        el_.ref_modify_classes(
                                            &[
                                                (
                                                    &style_export::class_state_selected().value,
                                                    is_selected(&*mode.borrow(), message_id, client_key),
                                                ),
                                            ],
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
                                let Some(new_mode) = reply_mode(&chat_state, &message_id, &client_key) else {
                                    return;
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
