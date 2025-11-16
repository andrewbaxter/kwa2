use {
    crate::{
        api::req_get,
        chat_entry::{
            ChatFeedId,
            ChatMode,
            ChatModeMessage,
            ChatTime,
            ChatTimeId,
        },
        chat_feed_channel::ChannelFeed,
        chat_feed_controls::FeedControls,
        chat_feed_outbox::OutboxFeed,
        infinite::Infinite,
        js::style_export,
        localdata::get_or_req_api_channel,
        pageutil::build_nol_chat_bar,
        state::{
            ministate_octothorpe,
            record_replace_ministate,
            spawn_rooted_log,
            state,
            ChannelFeedPair,
            Ministate,
            MinistateChannel,
            SessionStorageChatReset,
            SessionStorageChatResetSource,
            SESSIONSTORAGE_CHAT_RESET,
        },
    },
    flowcontrol::shed,
    gloo::storage::{
        SessionStorage,
        Storage,
    },
    jiff::Timestamp,
    lunk::{
        link,
        HistPrim,
        ProcessingContext,
    },
    rooting::{
        scope_any,
        El,
    },
    shared::interface::wire::c2s,
    std::cell::RefCell,
};

pub fn build(pc: &mut ProcessingContext, m: &MinistateChannel) -> El {
    // Restore initial state
    let reset_id = match &m.reset_id {
        Some(r) => Some(r.clone()),
        None => match SessionStorage::get::<SessionStorageChatReset>(SESSIONSTORAGE_CHAT_RESET) {
            Ok(v) => {
                match v.source {
                    SessionStorageChatResetSource::Channel(s) if s == m.id => {
                        Some(v.reset_id)
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
    let mode;
    match &reset_id {
        Some(id) => {
            mode = HistPrim::new(pc, ChatMode::ReplyMessage(ChatModeMessage::Channel(id.clone())));
        },
        None => {
            mode = HistPrim::new(pc, ChatMode::None);
        },
    }
    let ministate = HistPrim::new(pc, None);

    // Build inf
    let inf = Infinite::new(&pc.eg());
    inf.add_feed(
        ChatFeedId::Controls,
        FeedControls::new(mode.clone(), vec![("default".to_string(), m.id.clone())]),
    );
    let f_c = ChannelFeed::new(m.id.clone());
    inf.add_feed(ChatFeedId::Channel(m.id.clone()), f_c.clone());
    let f_o = OutboxFeed::new(m.id.clone());
    inf.add_feed(ChatFeedId::Outbox(m.id.clone()), f_o.clone());
    state().channel_feeds.borrow_mut().clear();
    state().channel_feeds.borrow_mut().extend([(m.id.clone(), ChannelFeedPair {
        channel: f_c,
        outbox: f_o,
    })].into_iter());
    *state().current_chat.borrow_mut() = Some(inf.clone());

    // Event handling
    let mut own = vec![];
    own.push(
        scope_any(
            link!((pc = pc), (mode = mode.clone()), (ministate = ministate.clone()), (bg_seek = RefCell::new(None)) {
                let new_ministate;
                match &*mode.borrow() {
                    ChatMode::None => new_ministate = None,
                    ChatMode::MessageChannelSelect => new_ministate = None,
                    ChatMode::TopMessage(_) => new_ministate = None,
                    ChatMode::ReplyMessage(t) => match t {
                        ChatModeMessage::Channel(m) => {
                            new_ministate = Some(m.clone());
                        },
                        ChatModeMessage::Outbox(_) => {
                            new_ministate = None;
                        },
                    },
                };
                if new_ministate != old_ministate {
                    record_replace_ministate(&state().log, Ministate::Channel(MinistateChannel {
                        id: id.clone(),
                        reset_id: ministate.get(),
                    }));
                    old_ministate.borrow_mut() = new_ministate;
                }
            }),
        ),
    );

    // Lazy initialization
    if let Some(reset_id) = reset_id {
        own.push(scope_any(spawn_rooted_log("Looking up time to seek chat", {
            let inf = inf.weak();
            let reset_id = reset_id.clone();
            let eg = pc.eg();
            async move {
                let reset_time = shed!{
                    let Some(initial_page) =
                        req_get(
                            &state().env.base_url,
                            c2s::SnapPageContaining { id: reset_id.clone() },
                        ).await? else {
                            break ChatTime {
                                stamp: Timestamp::now(),
                                id: ChatTimeId::None,
                            };
                        };
                    let Some(found) = initial_page.messages.iter().find(|m| m.original_id == reset_id) else {
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
                let Some(inf) = inf.upgrade() else {
                    return Ok(());
                };
                inf.jump_to(&reset_time);
                mode.set(pc, ChatMode::ReplyMessage(reset_time));
                return Ok(());
            }
        })));
    }
    inf.padding_pre_el().ref_push(style_export::cont_chat_bar(style_export::ContChatBarArgs {
        back_link: ministate_octothorpe(&Ministate::Top),
        center: build_nol_chat_bar(&Ministate::Top, get_or_req_api_channel(&m.id, true), {
            let id = m.id.clone();
            move |local_channel| style_export::leaf_chat_bar_center(style_export::LeafChatBarCenterArgs {
                text: local_channel.res.memo_short.clone(),
                link: Some(ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                    id: id.clone(),
                    reset_id: None,
                }))),
            }).root
        }),
        right: None,
    }).root);
    return inf.el().own(move |_| own);
}
