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
        chat_entry::{
            ChatEntryControls,
            ChatEntryMessageInternal,
        },
        js::{
            ElExt,
            style_export,
        },
        opfs::opfs_write_json,
        outbox::{
            new_message_client_id,
            OPFS_FILENAME_MAIN,
            OutboxMessage,
            OutboxMessageRel,
            OutboxMessageReplyTo,
            opfs_outbox_channel_dir,
            opfs_outbox_message_dir,
        },
        state::{
            spawn_log,
            state,
        },
    },
    gloo_events::EventListenerOptions,
    lunk::{
        HistPrim,
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        scope_any,
    },
    shared::interface::shared::QualifiedChannelId,
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm_bindgen::JsCast,
    web_sys::KeyboardEvent,
};

fn configure_message_text_send_on_enter(text: &El, send: &El) {
    text.ref_on_with_options("keydown", EventListenerOptions::enable_prevent_default(), {
        let send = send.weak();
        move |ev| {
            let ev = ev.dyn_ref::<KeyboardEvent>().unwrap();
            if ev.key() != "Enter" || ev.shift_key() || ev.is_composing() {
                return;
            }
            ev.prevent_default();
            let Some(send) = send.upgrade() else {
                return;
            };
            send.html().click();
        }
    });
}

fn configure_send(
    pc: &mut ProcessingContext,
    send: &El,
    text: &El,
    mode: &HistPrim<ChatMode>,
    chat_state2: &Rc<ChatState2>,
    channel: QualifiedChannelId,
    own_identity: Identity,
) {
    send.ref_on("click", {
        let text = text.clone();
        let mode = mode.clone();
        let eg = pc.eg();
        let chat_state2 = chat_state2.clone();
        move |_| {
            let Some(body) = text.raw().text_content().filter(|t| !t.is_empty()) else {
                return;
            };
            let client_id = match new_message_client_id() {
                Ok(c) => c,
                Err(e) => {
                    state().log.log(&e);
                    return;
                },
            };
            let reply_to = match &*mode.borrow() {
                ChatMode::ReplyMessage(c) => Some(match &c.target {
                    ChatModeReplyMessageTarget::Channel(m) => OutboxMessageReplyTo::Channel(m.clone()),
                    ChatModeReplyMessageTarget::Outbox(m) => OutboxMessageReplyTo::Outbox(m.message.clone()),
                }),
                _ => None,
            };
            text.ref_text("");
            let outbox_feed =
                chat_state2
                    .channel_lookup
                    .borrow()
                    .get(&channel)
                    .and_then(|f| f.outboxes.borrow().get(&own_identity).cloned());
            if let Some(f) = outbox_feed {
                f.add_sent(client_id.clone(), body.clone());
            }
            spawn_log("Saving sent message to outbox", {
                let channel = channel.clone();
                let own_identity = own_identity.clone();
                let message = OutboxMessage {
                    rel: match &reply_to {
                        None => OutboxMessageRel::None,
                        Some(r) => OutboxMessageRel::ReplyTo(r.clone()),
                    },
                    client_id: client_id.clone(),
                    body: body,
                };
                async move {
                    let opfs_channel_dir = opfs_outbox_channel_dir(&own_identity, &channel).await;
                    let opfs_message_dir = opfs_outbox_message_dir(&opfs_channel_dir, &message.client_id).await;
                    opfs_write_json(&opfs_message_dir, OPFS_FILENAME_MAIN, message)
                        .await
                        .map_err(|e| format!("Error writing message to opfs: [{}]", e))?;
                    trigger_push();
                    return Ok(());
                }
            });
            eg.event(|pc| {
                mode.set(pc, ChatMode::ReplyMessage(ChatModeReplyMessage {
                    target: ChatModeReplyMessageTarget::Outbox(ChatModeReplyMessageTargetOutbox {
                        channel: channel.clone(),
                        message: client_id.clone(),
                    }),
                    own_identity: own_identity.clone(),
                }));
            }).unwrap();
        }
    });
}

pub fn build_chat_entry_controls(pc: &mut ProcessingContext, m: &ChatEntryControls) -> El {
    let out = style_export::cont_chat_entry_mode_controls().root;
    out.ref_own(
        |out| link!(
            (pc = pc),
            (channels = m.state.channels_meta.clone()),
            (),
            (out = out.weak(), mode = m.mode.clone()) {
                let out = out.upgrade()?;
                out.ref_clear();
                if channels.borrow().is_empty() {
                    return None;
                }
                out.ref_push(style_export::leaf_chat_entry_mode_controls_button_new_message().root.on("click", {
                    let eg = pc.eg();
                    let channels = channels.clone();
                    let mode = mode.clone();
                    move |_| eg.event(|pc| {
                        let channels = channels.borrow();
                        if channels.len() > 1 {
                            mode.set(pc, ChatMode::MessageChannelSelect);
                        } else if let Some(channel) = channels.get(0) {
                            mode.set(pc, ChatMode::TopMessage(ChatModeTopMessage {
                                channel: channel.2.clone(),
                                own_identity: channel.1.clone(),
                            }));
                        }
                    }).unwrap()
                }));
            }
        ),
    );
    return out;
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
            (
                chat_state = chat_state,
                chat_state2 = chat_state2,
                inf_post = inf_post.weak(),
                menu_link = RefCell::new(None),
                reply_controls = RefCell::new(None)
            ) {
                let inf_post = inf_post.upgrade()?;
                let new_reply_controls = match &*mode.borrow() {
                    ChatMode::ReplyMessage(c) => Some((c.target.channel().clone(), c.own_identity.clone())),
                    _ => None,
                };
                if new_reply_controls.is_some() && *reply_controls.borrow() == new_reply_controls {
                    return None;
                }
                *reply_controls.borrow_mut() = new_reply_controls;
                *menu_link.borrow_mut() = None;
                inf_post.ref_clear();
                let inf_post_replace;
                let mut focus = None;
                match &*mode.borrow() {
                    ChatMode::None => {
                        inf_post_replace = vec![];
                    },
                    ChatMode::MessageChannelSelect => {
                        *menu_link.borrow_mut() =
                            Some(
                                scope_any(
                                    link!(
                                        (pc = pc),
                                        (channels = chat_state.channels_meta.clone()),
                                        (),
                                        (mode = mode.clone(), inf_post = inf_post.weak()) {
                                            let inf_post = inf_post.upgrade()?;
                                            inf_post.ref_clear();
                                            let mut children = vec![];
                                            for channel in &*channels.borrow() {
                                                children.push(
                                                    style_export::leaf_chat_controls_bar_mode_menu_button_new_message()
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
                                            inf_post.ref_push(
                                                style_export::cont_chat_controls_bar_mode_menu(
                                                    style_export::ContChatControlsBarModeMenuArgs {
                                                        children: children
                                                    },
                                                ).root,
                                            );
                                        }
                                    ),
                                ),
                            );
                        inf_post_replace = vec![];
                    },
                    ChatMode::TopMessage(c) => {
                        let controls = style_export::leaf_chat_controls_bar_mode_message();
                        controls.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                mode.set(pc, ChatMode::None);
                            }).unwrap()
                        });
                        configure_message_text_send_on_enter(&controls.text, &controls.send);
                        focus = Some(controls.text.clone());
                        configure_send(
                            pc,
                            &controls.send,
                            &controls.text,
                            &mode,
                            &chat_state2,
                            c.channel.clone(),
                            c.own_identity.clone(),
                        );
                        inf_post_replace = vec![controls.root];
                    },
                    ChatMode::ReplyMessage(c) => {
                        let controls = style_export::leaf_chat_controls_bar_mode_message();
                        controls.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                mode.set(pc, ChatMode::None);
                            }).unwrap()
                        });
                        configure_message_text_send_on_enter(&controls.text, &controls.send);
                        focus = Some(controls.text.clone());
                        configure_send(
                            pc,
                            &controls.send,
                            &controls.text,
                            &mode,
                            &chat_state2,
                            c.target.channel().clone(),
                            c.own_identity.clone(),
                        );
                        inf_post_replace = vec![controls.root];
                    },
                    ChatMode::EditMessage(c) => {
                        let controls = style_export::leaf_chat_controls_bar_mode_edit();
                        controls.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            let reply_mode = ChatMode::ReplyMessage(ChatModeReplyMessage {
                                target: ChatModeReplyMessageTarget::Channel(c.target.clone()),
                                own_identity: c.own_identity.clone(),
                            });
                            move |_| eg.event(|pc| {
                                mode.set(pc, reply_mode.clone());
                            }).unwrap()
                        });
                        controls.text.ref_text(&c.body);
                        configure_message_text_send_on_enter(&controls.text, &controls.send);
                        focus = Some(controls.text.clone());
                        controls.send.ref_on("click", {
                            let text = controls.text.clone();
                            let edit = c.clone();
                            let eg = pc.eg();
                            let chat_state = Rc::downgrade(chat_state);
                            move |_| {
                                let Some(chat_state) = chat_state.upgrade() else {
                                    return;
                                };
                                let Some(body) = text.raw().text_content().filter(|t| !t.is_empty()) else {
                                    return;
                                };
                                let reply_mode = ChatMode::ReplyMessage(ChatModeReplyMessage {
                                    target: ChatModeReplyMessageTarget::Channel(edit.target.clone()),
                                    own_identity: edit.own_identity.clone(),
                                });
                                if body == edit.body {
                                    eg.event(|pc| {
                                        chat_state.mode.set(pc, reply_mode);
                                    }).unwrap();
                                    return;
                                }
                                let client_id = match new_message_client_id() {
                                    Ok(c) => c,
                                    Err(e) => {
                                        state().log.log(&e);
                                        return;
                                    },
                                };
                                spawn_log("Saving message edit to outbox", {
                                    let channel = edit.target.channel.clone();
                                    let own_identity = edit.own_identity.clone();
                                    let message = OutboxMessage {
                                        rel: OutboxMessageRel::EditOf(edit.target.clone()),
                                        client_id: client_id,
                                        body: body.clone(),
                                    };
                                    async move {
                                        let opfs_channel_dir = opfs_outbox_channel_dir(&own_identity, &channel).await;
                                        let opfs_message_dir =
                                            opfs_outbox_message_dir(&opfs_channel_dir, &message.client_id).await;
                                        opfs_write_json(&opfs_message_dir, OPFS_FILENAME_MAIN, message)
                                            .await
                                            .map_err(|e| format!("Error writing message edit to opfs: [{}]", e))?;
                                        trigger_push();
                                        return Ok(());
                                    }
                                });
                                let entry =
                                    chat_state
                                        .entry_channel_lookup
                                        .borrow()
                                        .get(&edit.target)
                                        .and_then(|e| e.upgrade());
                                eg.event(|pc| {
                                    if let Some(entry) = entry {
                                        if let ChatEntryMessageInternal::Message(m) =
                                            &*entry.message().internal.borrow() {
                                            m.body.set(pc, body.clone());
                                        }
                                    }
                                    chat_state.mode.set(pc, reply_mode);
                                }).unwrap();
                            }
                        });
                        inf_post_replace = vec![controls.root];
                    },
                }
                inf_post.ref_extend(inf_post_replace);
                if let Some(focus) = focus {
                    focus.html().focus().unwrap();
                    let range = web_sys::window().unwrap().document().unwrap().create_range().unwrap();
                    range.select_node_contents(&focus.raw()).unwrap();
                    range.collapse_with_to_start(false);
                    let selection = web_sys::window().unwrap().get_selection().unwrap().unwrap();
                    selection.remove_all_ranges().unwrap();
                    selection.add_range(&range).unwrap();
                }
            }
        ),
    );
}
