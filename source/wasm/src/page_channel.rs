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
        chat_controls::build_controls,
        chat_feed_controls::FeedControls,
        chat_feed_outbox::OutboxFeed,
        chat_feed_server_outbox::ServerOutboxFeed,
        infinite::{
            Infinite,
            InfiniteEls,
        },
        js::{
            LogJsErr,
            style_export,
        },
        pageutil::build_nol_chat_bar,
        state::{
            CurrentChat,
            CurrentChatSource,
            LocalChannel1,
            Ministate,
            MinistateChannel,
            MinistateChannelSub,
            SESSIONSTORAGE_CHAT_RESET,
            SessionStorageChatReset,
            restore_track_chat_anchor,
            get_or_req_channel,
            ministate_octothorpe,
            record_replace_ministate,
            save_unread,
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

fn clear_unread(pc: &mut ProcessingContext, channel: &Rc<LocalChannel1>) {
    // Clear unread status
    let was_unread = *channel.unread.borrow();
    channel.unread.set(pc, false);
    if was_unread {
        save_unread();
    }

    // Propagate cleared unread status
    if let Some(group) = &*channel.group.borrow() {
        if let Some(group) = state().lookup_channelgroup.borrow().get(group) {
            shed!{
                'some_unread _;
                for c in &*group.children.borrow_values() {
                    if *c.unread.borrow() {
                        break 'some_unread;
                    }
                }
                group.unread.set(pc, false);
            }
        }
    }
    shed!{
        'some_unread _;
        for cocg in &*state().top.borrow_values() {
            match cocg {
                crate::state::LocalCocg::Channel(c) => {
                    if *c.unread.borrow() {
                        break 'some_unread;
                    }
                },
                crate::state::LocalCocg::ChannelGroup(cg) => {
                    if *cg.unread.borrow() {
                        break 'some_unread;
                    }
                },
            }
        }
        state().unread_any.set(pc, false);
    }
}

pub fn build(pc: &mut ProcessingContext, m: &MinistateChannel) -> El {
    if let Some(channel) = state().lookup_channel.borrow().get(&m.id) {
        clear_unread(pc, channel);
    }
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
    } 'no {
        let mode = HistPrim::new(pc, ChatMode::None);
        let chat_state =
            Rc::new(
                ChatState::new(
                    pc.eg(),
                    CurrentChatSource::Channel(m.id.clone()),
                    vec![("default".to_string(), m.own_identity.clone(), m.id.clone())],
                    mode.clone(),
                ),
            );
        inf = Infinite::new(&pc.eg(), InfiniteEls {
            center_spinner: style_export::leaf_chat_spinner_center().root,
            early_spinner: style_export::leaf_chat_spinner_early().root,
            late_spinner: style_export::leaf_chat_spinner_late().root,
        });
        inf.el().ref_classes(&[&style_export::class_page_chat().value]);
        inf.add_feed(ChatFeedId::Controls, FeedControls::new(mode.clone(), chat_state.clone()));
        let f_c = ChannelFeed::new(chat_state.clone(), m.id.clone());
        inf.add_feed(ChatFeedId::Channel(m.id.clone()), f_c.clone());
        let f_o = OutboxFeed::new(chat_state.clone(), m.own_identity.clone(), m.id.clone());
        inf.add_feed(ChatFeedId::Outbox((m.own_identity.clone(), m.id.clone())), f_o.clone());
        let f_so = ServerOutboxFeed::new(chat_state.clone(), m.own_identity.clone(), m.id.clone());
        inf.add_feed(ChatFeedId::ServerOutbox((m.own_identity.clone(), m.id.clone())), f_so.clone());
        let chat_state2 = Rc::new(ChatState2 { channel_lookup: Default::default() });
        chat_state2.channel_lookup.borrow_mut().clear();
        chat_state2.channel_lookup.borrow_mut().extend([(m.id.clone(), ChannelFeeds {
            channel: f_c,
            outboxes: RefCell::new([(m.own_identity.clone(), f_o)].into_iter().collect()),
            server_outboxes: RefCell::new([(m.own_identity.clone(), f_so)].into_iter().collect()),
        })].into_iter());
        *state().current_chat.borrow_mut() = Some(CurrentChat {
            source: CurrentChatSource::Channel(m.id.clone()),
            inf: inf.clone(),
            chat_state2: chat_state2.clone(),
        });
        let mut own = vec![];

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
        restore_track_chat_anchor(&inf, CurrentChatSource::Channel(m.id.clone()), reset_id.is_none());

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
                        match mode.borrow().focus() {
                            None => {
                                new_reset_id = None;
                                set_sticky = false;
                            },
                            Some(t) => match &t.target {
                                ChatModeReplyMessageTarget::Channel(m) => {
                                    new_reset_id = Some(m.clone());
                                    let found_time = chat_state.find_time_by_message_id(&m);
                                    if let Some(time) = found_time {
                                        if let Some(inf) = inf.upgrade() {
                                            inf.set_sticky(&time);
                                        }
                                        set_sticky = true;
                                    } else {
                                        *bg_seek.borrow_mut() =
                                            Some(spawn_rooted_log("Looking up chat seek location", {
                                                let inf = inf.clone();
                                                let reset_id = m.clone();
                                                async move {
                                                    let time = shed!{
                                                        let Some(found) =
                                                            req_get(
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
                                    let found_time =
                                        chat_state.find_time_by_client_id(
                                            &(id.clone(), t.own_identity.clone(), m.message.clone()),
                                        );
                                    if let Some(time) = found_time {
                                        if let Some(inf) = inf.upgrade() {
                                            inf.set_sticky(&time);
                                        }
                                        set_sticky = true;
                                    } else {
                                        *bg_seek.borrow_mut() =
                                            Some(spawn_rooted_log("Looking up chat seek location by outbox idem", {
                                                let inf = inf.clone();
                                                let reset_id = m.clone();
                                                async move {
                                                    let time = shed!{
                                                        let Some(found) = req_get(c2s::SnapByClientId {
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

        // Head bar
        let nol_channel = get_or_req_channel(&pc.eg(), &m.id, true);
        let calendar_link =
            style_export::leaf_chat_head_bar_right_calendar(
                style_export::LeafChatHeadBarRightCalendarArgs {
                    link: ministate_octothorpe(&Ministate::ChannelCalendar(MinistateChannelSub {
                        id: m.id.clone(),
                        own_identity: m.own_identity.clone(),
                    })),
                }
            ).root;
        let head_bar = build_nol_chat_bar(pc, &Ministate::Top, nol_channel.clone(), calendar_link, {
            let id = m.id.clone();
            let sender = m.own_identity.clone();
            move |pc, local_channel| {
                let center = style_export::leaf_chat_head_bar_center(style_export::LeafChatHeadBarCenterArgs {
                    text: local_channel.name.get(),
                    link: Some(ministate_octothorpe(&Ministate::ChannelMenu(MinistateChannelSub {
                        id: id.clone(),
                        own_identity: sender.clone(),
                    }))),
                }).root;
                center.ref_own(|e| link!((_pc = pc), (name = local_channel.name.clone()), (), (e = e.weak()) {
                    let e = e.upgrade()?;
                    e.ref_text(&*name.borrow());
                }));
                return center;
            }
        });
        own.push(nol_channel.then(pc, {
            let back_unread = head_bar.back_unread.weak();
            move |pc, channel| {
                let channel = match channel {
                    Ok(Some(x)) => x,
                    Ok(None) => {
                        return;
                    },
                    Err(e) => {
                        state().log.log(&format!("Error looking up channel: {}", e));
                        return;
                    },
                };
                clear_unread(pc, &channel);

                // Link back unread status
                if let Some(back_unread) = back_unread.upgrade() {
                    back_unread.ref_own(
                        |el_| link!((_pc = pc), (unread_any = state().unread_any.clone()), (), (el_ = el_.weak()) {
                            let el_ = el_.upgrade()?;
                            el_.ref_modify_classes(
                                &[(style_export::class_state_hidden().value.as_str(), !*unread_any.borrow())]
                            )
                        })
                    );
                };
            }
        }));

        // Assembly
        inf.padding_pre_el().ref_push(head_bar.root);
        build_controls(pc, &inf.padding_post_el(), chat_state.clone(), chat_state2.clone());
        inf.el().ref_own(move |_| own);
    });
    return inf.el();
}
