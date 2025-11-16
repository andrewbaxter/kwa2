use {
    crate::{
        api::req_get,
        chat_entry::{
            ChatMode,
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        chat_feed_channel::ChannelFeed,
        chat_feed_controls::FeedControls,
        chat_feed_outbox::OutboxFeed,
        infinite::Infinite,
        js::style_export,
        localdata::{
            get_or_req_api_channelgroup,
            get_stored_api_channels,
            req_api_channels,
        },
        pageutil::{
            build_nol_chat_bar,
        },
        state::{
            ministate_octothorpe,
            spawn_rooted_log,
            state,
            ChannelFeedPair,
            Ministate,
            MinistateChannelGroup,
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
        HistPrim,
        ProcessingContext,
    },
    rooting::{
        El,
    },
    shared::interface::{
        wire::c2s,
    },
    std::collections::{
        HashSet,
    },
};

pub fn build(pc: &mut ProcessingContext, m: &MinistateChannelGroup) -> El {
    let mut own = vec![];
    let controls_mode = HistPrim::new(pc, ChatMode::None);
    let inf = Infinite::new(&pc.eg());
    let f_controls = FeedControls::new(controls_mode.clone(), vec![]);
    inf.add_feed(ChatFeedId::Controls, f_controls.clone());
    state().channel_feeds.borrow_mut().clear();
    let mut old_channels = HashSet::new();
    for local_channel in get_stored_api_channels(None) {
        if local_channel.res.group.as_ref() != Some(&m.id) {
            continue;
        }
        f_controls.add_channel(local_channel.res.memo_short.clone(), local_channel.res.id.clone());
        let f_c = ChannelFeed::new(local_channel.res.id.clone());
        inf.add_feed(ChatFeedId::Channel(local_channel.res.id.clone()), f_c.clone());
        let f_o = OutboxFeed::new(local_channel.res.id.clone());
        inf.add_feed(ChatFeedId::Outbox(local_channel.res.id.clone()), f_o.clone());
        state().channel_feeds.borrow_mut().extend([(local_channel.res.id.clone(), ChannelFeedPair {
            channel: f_c,
            outbox: f_o,
        })].into_iter());
        old_channels.insert(local_channel.res.id);
    }
    own.push(spawn_rooted_log("Fetching channels in channel group", {
        let inf = inf.weak();
        let id = m.id.clone();
        async move {
            let local_channels = req_api_channels(None).await?;
            let Some(inf) = inf.upgrade() else {
                return Ok(());
            };
            for local_channel in local_channels {
                if local_channel.res.group.as_ref() != Some(&id) {
                    continue;
                }
                if old_channels.contains(&local_channel.res.id) {
                    continue;
                };
                let f_c = ChannelFeed::new(local_channel.res.id.clone());
                let f_o = OutboxFeed::new(local_channel.res.id.clone());
                inf.add_feed(ChatFeedId::Channel(local_channel.res.id.clone()), f_c.clone());
                inf.add_feed(ChatFeedId::Outbox(local_channel.res.id.clone()), f_o.clone());
                state().channel_feeds.borrow_mut().extend([(local_channel.res.id.clone(), ChannelFeedPair {
                    channel: f_c,
                    outbox: f_o,
                })].into_iter());
            }
            return Ok(());
        }
    }));
    *state().current_chat.borrow_mut() = Some(inf.clone());
    let reset_id = match &m.reset_id {
        Some(r) => Some(r.clone()),
        None => match SessionStorage::get::<SessionStorageChatReset>(SESSIONSTORAGE_CHAT_RESET) {
            Ok(v) => {
                match v.source {
                    SessionStorageChatResetSource::ChannelGroup(s) if s == m.id => {
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
    if let Some(reset_id) = reset_id {
        own.push(spawn_rooted_log("Looking up time to seek chat", {
            let inf = inf.weak();
            let reset_id = reset_id.clone();
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
                return Ok(());
            }
        }));
    }
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
    return inf.el();
}
