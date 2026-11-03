use {
    crate::{
        api::identity_image,
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
            MinistateMessageHistory,
            identity_portrait,
            ministate_octothorpe,
            state,
        },
    },
    lunk::{
        ProcessingContext,
        link,
    },
    flowcontrol::shed,
    rooting::El,
    std::rc::Rc,
};

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
                            if own_identity == *sender {
                                m_el
                                    .actions
                                    .ref_push(style_export::leaf_chat_entry_message_button_edit().root.on("click", {
                                        let eg = pc.eg();
                                        let chat_state = Rc::downgrade(chat_state);
                                        let message_id = message_id.clone();
                                        let own_identity = own_identity.clone();
                                        let body = m.body.clone();
                                        move |ev| {
                                            ev.stop_propagation();
                                            let Some(chat_state) = chat_state.upgrade() else {
                                                return;
                                            };
                                            eg.event(|pc| {
                                                chat_state.mode.set(pc, ChatMode::EditMessage(ChatModeEditMessage {
                                                    target: message_id.clone(),
                                                    own_identity: own_identity.clone(),
                                                    body: body.borrow().clone(),
                                                }));
                                            }).unwrap();
                                        }
                                    }));
                            }
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
