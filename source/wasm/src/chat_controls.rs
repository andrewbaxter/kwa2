use {
    crate::{
        chat::{
            ChatMode,
            ChatState,
        },
        chat_entry::{
            ChatEntryControls,
            ChatTime,
            ChatTimeId,
        },
        js::style_export,
        outbox::{
            opfs_channel_dir,
            OutboxMessage,
            OPFS_FILENAME_MAIN,
        },
        state::state,
    },
    jiff::Timestamp,
    lunk::{
        link,
        HistPrim,
        ProcessingContext,
    },
    rooting::El,
    shared::interface::shared::MessageIdem,
    std::rc::Rc,
};

pub fn build_chat_entry_controls(pc: &mut ProcessingContext, m: &ChatEntryControls) -> El {
    let out = style_export::leaf_chat_entry_controls();
    if !m.channels.borrow().is_empty() {
        out.root.ref_push(style_export::leaf_chat_entry_control_new_message().root.on("click", {
            let eg = pc.eg();
            let channels = m.channels.clone();
            let mode = m.group_mode.clone();
            move |_| eg.event(|pc| {
                if channels.borrow().len() > 1 {
                    mode.set(pc, ChatMode::MessageChannelSelect);
                } else {
                    mode.set(pc, ChatMode::TopMessage(channels.borrow().get(0).unwrap().1.clone()));
                }
            }).unwrap()
        }));
    }
    return out.root;
}

pub fn build_controls(pc: &mut ProcessingContext, inf_post: &El, chat_state: Rc<ChatState>) {
    inf_post.ref_own(
        |inf_post| link!(
            (pc = pc),
            (mode = chat_state.mode.clone()),
            (),
            (chat_state = chat_state.clone(), inf_post = inf_post.weak()) {
                let inf_post = inf_post.upgrade()?;
                inf_post.ref_clear();
                match &*mode.borrow() {
                    ChatMode::None => { },
                    ChatMode::MessageChannelSelect => {
                        let mut children = vec![];
                        for channel in &*chat_state.channels.borrow() {
                            children.push(
                                style_export::leaf_chat_controls_menu_button(
                                    style_export::LeafChatControlsMenuButtonArgs { text: channel.0.clone() },
                                )
                                    .root
                                    .on("click", {
                                        let mode = mode.clone();
                                        let eg = pc.eg();
                                        let channel = channel.1.clone();
                                        move |_| eg.event(|pc| {
                                            mode.set(pc, ChatMode::TopMessage(channel.clone()));
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
                        configure_button(controls.send, {
                            let text = controls.text.clone();
                            let channel = c.channel.clone();
                            let ident = c.identity.clone();
                            let stamp = Timestamp::now();
                            let idem = MessageIdem(stamp.to_string());
                            let mode = mode.clone();
                            let eg = pc.eg();
                            async move || {
                                let Some(text) = text.raw().text_content().filter(|t| !t.is_empty()) else {
                                    return;
                                };
                                let opfs_channel_dir = opfs_channel_dir(&channel).await;
                                let opfs_message_dir = opfs_message_dir(&opfs_channel_dir, idem);
                                opfs_write(&opfs_channel_dir, OPFS_FILENAME_MAIN, OutboxMessage {
                                    idem: idem.clone(),
                                    body: text,
                                });
                                let channels = state().channel_feeds.borrow();
                                if let Some(f) = channels.get(&channel) {
                                    f.outbox.notify(eg);
                                }
                                mode.set(pc, ChatMode::ReplyMessage(ChatTime {
                                    stamp: stamp,
                                    id: ChatTimeId::Outbox(idem.clone()),
                                }));
                            }
                        });
                        inf_post.ref_push(controls.root);
                    },
                }
            }
        ),
    );
}
