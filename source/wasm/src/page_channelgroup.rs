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
            ChatEntry,
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
        localdata::{
            LocalChannel,
            LocalIdentity,
            get_or_req_api_channelgroup,
            get_stored_api_channels,
            get_stored_api_identities,
            req_api_channels,
            req_api_identities,
        },
        pageutil::build_nol_chat_bar,
        state::{
            CurrentChat,
            CurrentChatSource,
            Ministate,
            MinistateChannelGroup,
            MinistateChannelGroupResetId,
            SESSIONSTORAGE_CHAT_RESET,
            SessionStorageChatReset,
            ministate_octothorpe,
            record_replace_ministate,
            set_page,
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
    shared::interface::{
        shared::{
            ChannelGroupId,
            QualifiedChannelId,
        },
        wire::c2s,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::{
            HashMap,
            HashSet,
        },
        rc::Rc,
    },
};

enum PopulateResult {
    Ok,
    TooMany,
}

fn populate(
    id: &ChannelGroupId,
    inf: &Infinite<ChatEntry>,
    chat_state: &Rc<ChatState>,
    chat_state2: &Rc<ChatState2>,
    local_identities: Vec<LocalIdentity>,
    local_channels: Vec<LocalChannel>,
) -> PopulateResult {
    struct TreeIdent {
        ident: Identity,
        memo_short: Option<String>,
    }

    struct Tree {
        memo_short: String,
        idents: Vec<TreeIdent>,
    }

    let mut idents = HashMap::new();
    for local_ident in local_identities {
        idents.insert(local_ident.res.id.clone(), local_ident.res.memo_short.clone());
    }
    let mut tree = HashMap::<QualifiedChannelId, Tree>::new();
    for local_channel in local_channels {
        if local_channel.res.group.as_ref() != Some(&id) {
            continue;
        }
        tree.entry(local_channel.res.id.clone()).or_insert_with(|| Tree {
            memo_short: local_channel.res.memo_short.clone(),
            idents: Default::default(),
        }).idents.push(TreeIdent {
            ident: local_channel.res.own_identity.clone(),
            memo_short: idents.get(&local_channel.res.own_identity).cloned(),
        });
    }
    chat_state.channels_meta.borrow_mut().clear();
    let mut too_many_channels = chat_state2.channel_lookup.borrow().keys().cloned().collect::<HashSet<_>>();
    for (channel, channel_entry) in tree {
        too_many_channels.remove(&channel);

        // Build channel list entry
        for ident in &channel_entry.idents {
            chat_state.channels_meta.borrow_mut().push((
                //. .
                if channel_entry.idents.len() == 1 {
                    channel_entry.memo_short.clone()
                } else {
                    format!(
                        "{} - {}",
                        channel_entry.memo_short,
                        ident.memo_short.clone().unwrap_or_else(|| ident.ident.to_string())
                    )
                },
                ident.ident.clone(),
                channel.clone(),
            ));
        }

        // Build feed
        let f_c = ChannelFeed::new(chat_state.clone(), channel.clone());
        inf.add_feed(ChatFeedId::Channel(channel.clone()), f_c.clone());
        let mut f_os = HashMap::new();
        for ident in &channel_entry.idents {
            let f_o = OutboxFeed::new(chat_state.clone(), ident.ident.clone(), channel.clone());
            inf.add_feed(ChatFeedId::Outbox((ident.ident.clone(), channel.clone())), f_o.clone());
            f_os.insert(ident.ident.clone(), f_o);
        }
        chat_state2.channel_lookup.borrow_mut().insert(channel.clone(), ChannelFeeds {
            channel: f_c,
            outboxes: RefCell::new(f_os),
        });
    }
    if !too_many_channels.is_empty() {
        return PopulateResult::TooMany;
    }
    return PopulateResult::Ok;
}

pub fn build(pc: &mut ProcessingContext, m: &MinistateChannelGroup) -> El {
    let mut own = vec![];

    // Build inf
    let chat_state;
    let chat_state2;
    let inf;
    superif!({
        let Some(current_chat) = state().current_chat.borrow().as_ref().cloned() else {
            break 'no;
        };
        match &current_chat.source {
            CurrentChatSource::Group(c) if *c == m.id => { },
            _ => break 'no,
        }
        inf = current_chat.inf;
        chat_state = current_chat.chat_state;
        chat_state2 = current_chat.chat_state2;
    } 'no {
        let mode = HistPrim::new(pc, ChatMode::None);
        chat_state = Rc::new(ChatState {
            channels_meta: RefCell::new(vec![]),
            entry_channel_lookup: Default::default(),
            entry_channel_lookup_by_client_id: Default::default(),
            entry_outbox_lookup: Default::default(),
            mode: mode.clone(),
        });
        chat_state2 = Rc::new(ChatState2 { channel_lookup: Default::default() });
        inf = Infinite::new(&pc.eg());
        inf.add_feed(ChatFeedId::Controls, FeedControls::new(mode.clone(), chat_state.clone()));
        *state().current_chat.borrow_mut() = Some(CurrentChat {
            source: CurrentChatSource::Group(m.id.clone()),
            inf: inf.clone(),
            chat_state: chat_state.clone(),
            chat_state2: chat_state2.clone(),
        });
    });

    // Populate/extend feeds
    if let PopulateResult::TooMany =
        populate(
            &m.id,
            &inf,
            &chat_state,
            &chat_state2,
            get_stored_api_identities(None),
            get_stored_api_channels(None),
        ) {
        // Wipe and restart
        *state().current_chat.borrow_mut() = None;
        return build(pc, m);
    }
    own.push(scope_any(spawn_rooted_log("Fetching channels in channel group", {
        let id = m.id.clone();
        let eg = pc.eg();
        let reset_id = m.reset_id.clone();
        async move {
            let local_channels = req_api_channels(None).await?;
            let local_idents = req_api_identities(None).await?;
            let Some(current) = state().current_chat.borrow().clone() else {
                return Ok(());
            };
            if let PopulateResult::TooMany =
                populate(&id, &current.inf, &current.chat_state, &current.chat_state2, local_idents, local_channels) {
                // Wipe and restart
                *state().current_chat.borrow_mut() = None;
                eg.event(|pc| {
                    set_page(build(pc, &MinistateChannelGroup {
                        id: id,
                        reset_id: reset_id.clone(),
                    }));
                }).unwrap();
            }
            return Ok(());
        }
    })));

    // Restore initial state
    let reset_id = match &m.reset_id {
        Some(r) => Some(r.clone()),
        None => match SessionStorage::get::<SessionStorageChatReset>(SESSIONSTORAGE_CHAT_RESET) {
            Ok(v) => {
                match v {
                    SessionStorageChatReset::ChannelGroup(s) if s.channel_group == m.id => {
                        Some(MinistateChannelGroupResetId {
                            own_identity: s.own_identity.clone(),
                            message: s.reset_id.clone(),
                        })
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
            target: ChatModeReplyMessageTarget::Channel(reset_id.message.clone()),
            own_identity: reset_id.own_identity.clone(),
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
                        ChatMode::ReplyMessage(mode) => match &mode.target {
                            ChatModeReplyMessageTarget::Channel(target) => {
                                new_reset_id = Some(MinistateChannelGroupResetId {
                                    own_identity: mode.own_identity.clone(),
                                    message: target.clone(),
                                });
                                if let Some(entry) = chat_state.entry_channel_lookup.borrow().get(&target) {
                                    if let Some(inf) = inf.upgrade() {
                                        inf.set_sticky(&entry.time());
                                    }
                                    set_sticky = true;
                                } else {
                                    *bg_seek.borrow_mut() = Some(spawn_rooted_log("Looking up chat seek location", {
                                        let inf = inf.clone();
                                        let reset_id = target.clone();
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
                            ChatModeReplyMessageTarget::Outbox(target) => {
                                new_reset_id = None;
                                if let Some(entry) =
                                    chat_state
                                        .entry_outbox_lookup
                                        .borrow()
                                        .get(
                                            &(
                                                target.channel.clone(),
                                                mode.own_identity.clone(),
                                                target.message.clone(),
                                            ),
                                        ) {
                                    if let Some(inf) = inf.upgrade() {
                                        inf.set_sticky(&entry.time());
                                    }
                                    set_sticky = true;
                                } else if let Some(entry) =
                                    chat_state
                                        .entry_channel_lookup_by_client_id
                                        .borrow()
                                        .get(
                                            &(
                                                target.channel.clone(),
                                                mode.own_identity.clone(),
                                                target.message.clone(),
                                            ),
                                        ) {
                                    if let Some(inf) = inf.upgrade() {
                                        inf.set_sticky(&entry.time());
                                    }
                                    set_sticky = true;
                                } else {
                                    *bg_seek.borrow_mut() =
                                        Some(spawn_rooted_log("Looking up chat seek location by outbox idem", {
                                            let inf = inf.clone();
                                            let reset_id = target.clone();
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
                        record_replace_ministate(&state().log, &Ministate::ChannelGroup(MinistateChannelGroup {
                            id: id.clone(),
                            reset_id: new_reset_id.clone(),
                        }));
                        *old_reset_id.borrow_mut() = new_reset_id.clone();
                        if let Some(reset_id) = &new_reset_id {
                            SessionStorage::set(
                                SESSIONSTORAGE_CHAT_RESET,
                                SessionStorageChatReset::Channel(reset_id.message.clone()),
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
        center: build_nol_chat_bar(&Ministate::Top, get_or_req_api_channelgroup(&m.id, true), {
            let id = m.id.clone();
            move |local_channel| style_export::leaf_chat_bar_center(style_export::LeafChatBarCenterArgs {
                text: local_channel.res.memo_short.clone(),
                link: Some(ministate_octothorpe(&Ministate::ChannelGroup(MinistateChannelGroup {
                    id: id.clone(),
                    reset_id: None,
                }))),
            }).root
        }),
        right: None,
    }).root);
    return inf.el().own(move |_| own);
}
