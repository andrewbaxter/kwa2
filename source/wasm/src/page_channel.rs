use {
    crate::{
        api::req_get,
        chat::{
            ChannelFeeds,
            ChatMode,
            ChatModeReplyMessage,
            ChatModeReplyMessageTarget,
            ChatState,
            ChatState2,
        },
        chat_entry::{
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        chat_feed_channel::ChannelFeed,
        chat_feed_controls::FeedControls,
        chat_feed_outbox::OutboxFeed,
        infinite::{
            Entry,
            Infinite,
        },
        js::{
            LogJsErr,
            style_export,
        },
        localdata::get_or_req_api_channel,
        pageutil::build_nol_chat_bar,
        state::{
            CurrentChat,
            CurrentChatSource,
            Ministate,
            MinistateChannel,
            SESSIONSTORAGE_CHAT_RESET,
            SessionStorageChatReset,
            ministate_octothorpe,
            record_replace_ministate,
            spawn_rooted_log,
            state,
        },
    },
    flowcontrol::{
        shed,
        superif,
    },
    gloo::storage::{
        SessionStorage,
        Storage,
    },
    jiff::Timestamp,
    lunk::{
        HistPrim,
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        scope_any,
    },
    shared::interface::wire::c2s,
    std::{
        cell::RefCell,
        rc::Rc,
    },
};

pub fn build(pc: &mut ProcessingContext, m: &MinistateChannel) -> El {
    let mut own = vec![];

    // Build inf
    let chat_state;
    let inf;
    superif!({
        let Some(current_chat) = state().current_chat.borrow().as_ref().cloned() else {
            break 'no;
        };
        match &current_chat.source {
            CurrentChatSource::Channel(c) if *c == m.id => { },
            _ => break 'no,
        }
        inf = current_chat.inf;
        chat_state = current_chat.chat_state;
    } 'no {
        let mode = HistPrim::new(pc, ChatMode::None);
        chat_state = Rc::new(ChatState {
            channels_meta: RefCell::new(vec![("default".to_string(), m.own_identity.clone(), m.id.clone())]),
            entry_channel_lookup: Default::default(),
            entry_channel_lookup_by_client_id: Default::default(),
            entry_outbox_lookup: Default::default(),
            mode: mode.clone(),
        });
        inf = Infinite::new(&pc.eg());
        inf.add_feed(ChatFeedId::Controls, FeedControls::new(mode.clone(), chat_state.clone()));
        let f_c = ChannelFeed::new(chat_state.clone(), m.id.clone());
        inf.add_feed(ChatFeedId::Channel(m.id.clone()), f_c.clone());
        let f_o = OutboxFeed::new(chat_state.clone(), m.own_identity.clone(), m.id.clone());
        inf.add_feed(ChatFeedId::Outbox((m.own_identity.clone(), m.id.clone())), f_o.clone());
        let chat_state2 = Rc::new(ChatState2 { channel_lookup: Default::default() });
        chat_state2.channel_lookup.borrow_mut().clear();
        chat_state2.channel_lookup.borrow_mut().extend([(m.id.clone(), ChannelFeeds {
            channel: f_c,
            outboxes: RefCell::new([(m.own_identity.clone(), f_o)].into_iter().collect()),
        })].into_iter());
        *state().current_chat.borrow_mut() = Some(CurrentChat {
            source: CurrentChatSource::Channel(m.id.clone()),
            inf: inf.clone(),
            chat_state: chat_state.clone(),
            chat_state2: chat_state2.clone(),
        });
    });

    // Restore initial state
    let reset_id = match &m.reset_id {
        Some(r) => Some(r.clone()),
        None => match SessionStorage::get::<SessionStorageChatReset>(SESSIONSTORAGE_CHAT_RESET) {
            Ok(v) => {
                match v {
                    SessionStorageChatReset::Channel(s) if s.channel == m.id => {
                        Some(s)
                    },
                    _ => {
                        SessionStorage::delete(SESSIONSTORAGE_CHAT_RESET);
                        None
                    },
                }
            },
            Err(_) => None,
        },
    };
    if let Some(reset_id) = &reset_id {
        chat_state.mode.set(pc, ChatMode::ReplyMessage(ChatModeReplyMessage {
            target: ChatModeReplyMessageTarget::Channel(reset_id.clone()),
            own_identity: m.own_identity.clone(),
        }));
    }

    // Event handling
    own.push(
        scope_any(
            link!(
                (_pc = pc),
                (mode = chat_state.mode.clone()),
                (),
                (
                    id = m.id.clone(),
                    sender = m.own_identity.clone(),
                    inf = inf.weak(),
                    old_reset_id = RefCell::new(None),
                    bg_seek = RefCell::new(None),
                    chat_state = chat_state.clone()
                ) {
                    let new_reset_id;
                    let set_sticky;
                    match &*mode.borrow() {
                        ChatMode::None => {
                            new_reset_id = None;
                            set_sticky = false;
                        },
                        ChatMode::MessageChannelSelect => {
                            new_reset_id = None;
                            set_sticky = false;
                        },
                        ChatMode::TopMessage(_) => {
                            new_reset_id = None;
                            set_sticky = false;
                        },
                        ChatMode::ReplyMessage(t) => match &t.target {
                            ChatModeReplyMessageTarget::Channel(m) => {
                                new_reset_id = Some(m.clone());
                                if let Some(entry) = chat_state.entry_channel_lookup.borrow().get(&m) {
                                    if let Some(inf) = inf.upgrade() {
                                        inf.set_sticky(&entry.time());
                                    }
                                    set_sticky = true;
                                } else {
                                    *bg_seek.borrow_mut() = Some(spawn_rooted_log("Looking up chat seek location", {
                                        let inf = inf.clone();
                                        let reset_id = m.clone();
                                        async move {
                                            let time = shed!{
                                                let Some(found) =
                                                    req_get(
                                                        &state().env.base_url,
                                                        c2s::SnapById { id: reset_id.clone() },
                                                    ).await? else {
                                                        break ChatTime {
                                                            stamp: Timestamp::now(),
                                                            id: ChatTimeId::None,
                                                        };
                                                    };
                                                break ChatTime {
                                                    stamp: found.original_receive_time,
                                                    id: ChatTimeId::Channel(found.offset),
                                                };
                                            };
                                            if let Some(inf) = inf.upgrade() {
                                                inf.set_sticky(&time);
                                            }
                                            return Ok(());
                                        }
                                    }));
                                    set_sticky = false;
                                }
                            },
                            ChatModeReplyMessageTarget::Outbox(m) => {
                                new_reset_id = None;
                                if let Some(entry) =
                                    chat_state
                                        .entry_outbox_lookup
                                        .borrow()
                                        .get(&(id.clone(), t.own_identity.clone(), m.message.clone())) {
                                    if let Some(inf) = inf.upgrade() {
                                        inf.set_sticky(&entry.time());
                                    }
                                    set_sticky = true;
                                } else if let Some(entry) =
                                    chat_state
                                        .entry_channel_lookup_by_client_id
                                        .borrow()
                                        .get(&(id.clone(), t.own_identity.clone(), m.message.clone())) {
                                    if let Some(inf) = inf.upgrade() {
                                        inf.set_sticky(&entry.time());
                                    }
                                    set_sticky = true;
                                } else {
                                    *bg_seek.borrow_mut() =
                                        Some(spawn_rooted_log("Looking up chat seek location by outbox idem", {
                                            let inf = inf.clone();
                                            let reset_id = m.clone();
                                            async move {
                                                let time = shed!{
                                                    let Some(found) =
                                                        req_get(&state().env.base_url, c2s::SnapByClientId {
                                                            channel: reset_id.channel.clone(),
                                                            client_id: reset_id.message.clone(),
                                                        }).await? else {
                                                            break ChatTime {
                                                                stamp: Timestamp::now(),
                                                                id: ChatTimeId::None,
                                                            };
                                                        };
                                                    break ChatTime {
                                                        stamp: found.original_receive_time,
                                                        id: ChatTimeId::Channel(found.offset),
                                                    };
                                                };
                                                if let Some(inf) = inf.upgrade() {
                                                    inf.set_sticky(&time);
                                                }
                                                return Ok(());
                                            }
                                        }));
                                    set_sticky = false;
                                }
                            },
                        },
                    };
                    if !set_sticky {
                        if let Some(inf) = inf.upgrade() {
                            inf.clear_sticky();
                        }
                    }
                    if new_reset_id != *old_reset_id.borrow() {
                        record_replace_ministate(&state().log, &Ministate::Channel(MinistateChannel {
                            id: id.clone(),
                            own_identity: sender.clone(),
                            reset_id: new_reset_id.clone(),
                        }));
                        *old_reset_id.borrow_mut() = new_reset_id.clone();
                        if let Some(reset_id) = &new_reset_id {
                            SessionStorage::set(
                                SESSIONSTORAGE_CHAT_RESET,
                                SessionStorageChatReset::Channel(reset_id.clone()),
                            ).log(&state().log, &"Error storing chat reset");
                        } else {
                            SessionStorage::delete(SESSIONSTORAGE_CHAT_RESET);
                        }
                    }
                }
            ),
        ),
    );

    // Assembly
    inf.padding_pre_el().ref_push(style_export::cont_chat_bar(style_export::ContChatBarArgs {
        back_link: ministate_octothorpe(&Ministate::Top),
        center: build_nol_chat_bar(&Ministate::Top, get_or_req_api_channel(&m.id, true), {
            let id = m.id.clone();
            let sender = m.own_identity.clone();
            move |local_channel| style_export::leaf_chat_bar_center(style_export::LeafChatBarCenterArgs {
                text: local_channel.res.memo_short.clone(),
                link: Some(ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                    id: id.clone(),
                    own_identity: sender.clone(),
                    reset_id: None,
                }))),
            }).root
        }),
        right: None,
    }).root);
    return inf.el().own(move |_| own);
}
