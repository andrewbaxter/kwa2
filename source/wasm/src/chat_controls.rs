use {
    crate::{
        background::trigger_push,
        chat::{
            ChatMode,
            ChatModeReplyMessage,
            ChatModeReplyMessageTarget,
            ChatModeReplyMessageTargetOutbox,
            ChatModeTopMessage,
            ChatState,
            ChatState2,
        },
        chat_entry::ChatEntryControls,
        js::{
            configure_async_button_once,
            style_export,
        },
        outbox::{
            OPFS_FILENAME_MAIN,
            OutboxMessage,
            OutboxMessageReplyTo,
            opfs_outbox_channel_dir,
            opfs_outbox_message_dir,
            opfs_write_json,
        },
        state::state,
    },
    jiff::Timestamp,
    lunk::{
        ProcessingContext,
        link,
    },
    rooting::El,
    shared::interface::shared::MessageClientId,
    std::rc::Rc,
};

pub fn build_chat_entry_controls(pc: &mut ProcessingContext, m: &ChatEntryControls) -> El {
    let out = style_export::leaf_chat_entry_controls();
    if !m.state.channels_meta.borrow().is_empty() {
        out.root.ref_push(style_export::leaf_chat_entry_control_new_message().root.on("click", {
            let eg = pc.eg();
            let channels = m.state.channels_meta.clone();
            let mode = m.mode.clone();
            move |_| eg.event(|pc| {
                if channels.borrow().len() > 1 {
                    mode.set(pc, ChatMode::MessageChannelSelect);
                } else {
                    let channels = channels.borrow();
                    let channel = channels.get(0).unwrap();
                    mode.set(pc, ChatMode::TopMessage(ChatModeTopMessage {
                        channel: channel.2.clone(),
                        own_identity: channel.1.clone(),
                    }));
                }
            }).unwrap()
        }));
    }
    return out.root;
}

pub fn build_controls(
    pc: &mut ProcessingContext,
    inf_post: &El,
    chat_state: Rc<ChatState>,
    chat_state2: Rc<ChatState2>,
) {
    inf_post.ref_own(
        |inf_post| link!(
            (pc = pc),
            (mode = chat_state.mode.clone()),
            (),
            (chat_state = chat_state, chat_state2 = chat_state2, inf_post = inf_post.weak()) {
                let inf_post = inf_post.upgrade()?;
                inf_post.ref_clear();
                match &*mode.borrow() {
                    ChatMode::None => { },
                    ChatMode::MessageChannelSelect => {
                        let mut children = vec![];
                        for channel in &*chat_state.channels_meta.borrow() {
                            children.push(
                                style_export::leaf_chat_controls_menu_button(
                                    style_export::LeafChatControlsMenuButtonArgs { text: channel.0.clone() },
                                )
                                    .root
                                    .on("click", {
                                        let mode = mode.clone();
                                        let eg = pc.eg();
                                        let sender = channel.1.clone();
                                        let channel = channel.2.clone();
                                        move |_| eg.event(|pc| {
                                            mode.set(pc, ChatMode::TopMessage(ChatModeTopMessage {
                                                channel: channel.clone(),
                                                own_identity: sender.clone(),
                                            }));
                                        }).unwrap()
                                    }),
                            );
                        }
                    },
                    ChatMode::TopMessage(c) => {
                        let controls = style_export::leaf_chat_controls_message();
                        controls.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                mode.set(pc, ChatMode::None);
                            }).unwrap()
                        });
                        configure_async_button_once(&controls.send, {
                            let text = controls.text.clone();
                            let c = c.clone();
                            let client_id = MessageClientId::from_timestamp(Timestamp::now());
                            let mode = mode.clone();
                            let eg = pc.eg();
                            let chat_state2 = chat_state2.clone();
                            async move || {
                                let Some(text) = text.raw().text_content().filter(|t| !t.is_empty()) else {
                                    return;
                                };
                                let opfs_channel_dir = opfs_outbox_channel_dir(&c.own_identity, &c.channel).await;
                                let opfs_message_dir = opfs_outbox_message_dir(&opfs_channel_dir, &client_id).await;
                                if let Err(e) = opfs_write_json(&opfs_message_dir, OPFS_FILENAME_MAIN, OutboxMessage {
                                    reply_to: None,
                                    client_id: client_id.clone(),
                                    body: text,
                                }).await {
                                    state().log.log(&format!("Error writing message to opfs: [{}]", e));
                                    return;
                                };
                                trigger_push();
                                let channels = chat_state2.channel_lookup.borrow();
                                if let Some(f) = channels.get(&c.channel) {
                                    let outboxes = f.outboxes.borrow();
                                    if let Some(f) = outboxes.get(&c.own_identity) {
                                        f.notify(eg.clone());
                                    }
                                }
                                eg.event(|pc| {
                                    mode.set(pc, ChatMode::ReplyMessage(ChatModeReplyMessage {
                                        target: ChatModeReplyMessageTarget::Outbox(ChatModeReplyMessageTargetOutbox {
                                            channel: c.channel.clone(),
                                            message: client_id.clone(),
                                        }),
                                        own_identity: c.own_identity.clone(),
                                    }));
                                }).unwrap();
                            }
                        });
                        inf_post.ref_push(controls.root);
                    },
                    ChatMode::ReplyMessage(c) => {
                        let controls = style_export::leaf_chat_controls_message();
                        controls.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                mode.set(pc, ChatMode::None);
                            }).unwrap()
                        });
                        configure_async_button_once(&controls.send, {
                            let text = controls.text.clone();
                            let c = c.clone();
                            let client_id = MessageClientId::from_timestamp(Timestamp::now());
                            let mode = mode.clone();
                            let eg = pc.eg();
                            let chat_state2 = chat_state2.clone();
                            async move || {
                                let Some(text) = text.raw().text_content().filter(|t| !t.is_empty()) else {
                                    return;
                                };
                                let channel = match &c.target {
                                    ChatModeReplyMessageTarget::Channel(m) => &m.channel,
                                    ChatModeReplyMessageTarget::Outbox(m) => &m.channel,
                                };
                                let opfs_channel_dir = opfs_outbox_channel_dir(&c.own_identity, channel).await;
                                let opfs_message_dir = opfs_outbox_message_dir(&opfs_channel_dir, &client_id).await;
                                if let Err(e) = opfs_write_json(&opfs_message_dir, OPFS_FILENAME_MAIN, OutboxMessage {
                                    reply_to: Some(match &c.target {
                                        ChatModeReplyMessageTarget::Channel(m) => OutboxMessageReplyTo::Channel(
                                            m.clone(),
                                        ),
                                        ChatModeReplyMessageTarget::Outbox(m) => OutboxMessageReplyTo::Outbox(
                                            m.message.clone(),
                                        ),
                                    }),
                                    client_id: client_id.clone(),
                                    body: text,
                                }).await {
                                    state().log.log(&format!("Error writing message to opfs: [{}]", e));
                                    return;
                                }
                                trigger_push();
                                let channels = chat_state2.channel_lookup.borrow();
                                if let Some(f) = channels.get(channel) {
                                    let outboxes = f.outboxes.borrow();
                                    if let Some(f) = outboxes.get(&c.own_identity) {
                                        f.notify(eg.clone());
                                    }
                                }
                                eg.event(|pc| {
                                    mode.set(pc, ChatMode::ReplyMessage(ChatModeReplyMessage {
                                        target: ChatModeReplyMessageTarget::Outbox(ChatModeReplyMessageTargetOutbox {
                                            channel: channel.clone(),
                                            message: client_id.clone(),
                                        }),
                                        own_identity: c.own_identity.clone(),
                                    }));
                                }).unwrap();
                            }
                        });
                        inf_post.ref_push(controls.root);
                    },
                }
            }
        ),
    );
}
