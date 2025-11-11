use {
    crate::{
        api::req_get,
        chat_entry::{
            ChatFeedId,
            ChatFeedIdSub,
            ChatTime,
            ChatTimeId,
        },
        chat_feed_channel::ChannelFeed,
        chat_feed_outbox::OutboxFeed,
        infinite::Infinite,
        js::style_export,
        localdata::{
            get_or_req_api_channelgroup,
            get_stored_api_channels,
            req_api_channels,
            NowOrLater,
        },
        pageutil::{
            build_nol_chat,
            LazyPage,
        },
        state::{
            ministate_octothorpe,
            state,
            Ministate,
            MinistateChannelGroup,
        },
    },
    jiff::Timestamp,
    lunk::ProcessingContext,
    rooting::{
        spawn_rooted,
        El,
    },
    shared::interface::{
        shared::{
            ChannelGroupId,
            QualifiedMessageId,
        },
        wire::c2s,
    },
    std::collections::HashSet,
};

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId, reset_id: &Option<QualifiedMessageId>) -> El {
    let nol0 = get_or_req_api_channelgroup(id, true);
    let nol;
    match reset_id {
        Some(r) => {
            nol = NowOrLater::Later(spawn_rooted({
                let reset_id = r.clone();
                async move {
                    let nol = match nol0 {
                        NowOrLater::Now(v) => v,
                        NowOrLater::Later(v) => match v.await.map_err(|e| e.to_string())?? {
                            Some(v) => v,
                            None => {
                                return Ok(None);
                            },
                        },
                    };
                    let Some(initial_page) =
                        req_get(
                            &state().env.base_url,
                            c2s::SnapPageContaining { id: reset_id.clone() },
                        ).await? else {
                            return Ok(Some((nol, ChatTime {
                                stamp: Timestamp::now(),
                                id: ChatTimeId::Seek,
                            })));
                        };
                    let Some(found) = initial_page.messages.iter().find(|m| m.original_id == reset_id) else {
                        return Ok(Some((nol, ChatTime {
                            stamp: Timestamp::now(),
                            id: ChatTimeId::Seek,
                        })));
                    };
                    return Ok(Some((nol, ChatTime {
                        stamp: found.original_receive_time,
                        id: ChatTimeId::Channel(found.offset),
                    })));
                }
            }));
        },
        None => {
            nol = nol0.map(|local_channel| (local_channel, ChatTime {
                stamp: Timestamp::now(),
                id: ChatTimeId::Seek,
            }));
        },
    }
    return build_nol_chat(&Ministate::Top, nol, {
        let eg = pc.eg();
        let reset_id = reset_id.clone();
        move |(local_channelgroup, reset_time)| {
            let inf = Infinite::new(&eg, reset_time);
            let mut old_channels = HashSet::new();
            for local_channel in get_stored_api_channels(None) {
                if local_channel.res.group.as_ref() != Some(&local_channelgroup.res.id) {
                    continue;
                }
                inf.add_feed(
                    ChatFeedId(local_channel.res.id.clone(), ChatFeedIdSub::Channel),
                    ChannelFeed::new(local_channel.res.id.clone()),
                );
                inf.add_feed(
                    ChatFeedId(local_channel.res.id.clone(), ChatFeedIdSub::Outbox),
                    OutboxFeed::new(local_channel.res.id.clone()),
                );
                old_channels.insert(local_channel.res.id);
            }
            let inf_el = inf.el();
            inf_el.ref_own(|_| spawn_rooted({
                let inf = inf.weak();
                async move {
                    let local_channels = match req_api_channels(None).await {
                        Ok(c) => c,
                        Err(e) => {
                            state().log.log(&format!("Error refreshing channels: {}", e));
                            return;
                        },
                    };
                    let Some(inf) = inf.upgrade() else {
                        return;
                    };
                    for local_channel in local_channels {
                        if local_channel.res.group.as_ref() != Some(&local_channelgroup.res.id) {
                            continue;
                        }
                        if old_channels.contains(&local_channel.res.id) {
                            continue;
                        };
                        inf.add_feed(
                            ChatFeedId(local_channel.res.id.clone(), ChatFeedIdSub::Channel),
                            ChannelFeed::new(local_channel.res.id.clone()),
                        );
                        inf.add_feed(
                            ChatFeedId(local_channel.res.id.clone(), ChatFeedIdSub::Outbox),
                            OutboxFeed::new(local_channel.res.id.clone()),
                        );
                    }
                }
            }));
            *state().current_chat.borrow_mut() = Some(inf.clone());
            LazyPage {
                center: style_export::leaf_chat_bar_center(style_export::LeafChatBarCenterArgs {
                    text: local_channelgroup.res.memo_short.clone(),
                    link: Some(ministate_octothorpe(&Ministate::ChannelGroupMenu(MinistateChannelGroup {
                        channelgroup: local_channelgroup.res.id.clone(),
                        reset: reset_id.clone(),
                    }))),
                }).root,
                body: vec![inf.el()],
            }
        }
    });
}
