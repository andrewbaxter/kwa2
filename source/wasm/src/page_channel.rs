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
            get_or_req_api_channel,
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
            MinistateChannel,
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
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s,
    },
};

pub fn build(pc: &mut ProcessingContext, id: &QualifiedChannelId, reset_id: &Option<QualifiedMessageId>) -> El {
    let nol0 = get_or_req_api_channel(id, true);
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
                            None => return Ok(None),
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
        let id = id.clone();
        let reset_id = reset_id.clone();
        let eg = pc.eg();
        move |(local_channel, reset_time)| {
            let inf = Infinite::new(&eg, reset_time);
            inf.add_feed(ChatFeedId(id.clone(), ChatFeedIdSub::Channel), ChannelFeed::new(id.clone()));
            inf.add_feed(ChatFeedId(id.clone(), ChatFeedIdSub::Outbox), OutboxFeed::new(id.clone()));
            *state().current_chat.borrow_mut() = Some(inf.clone());
            LazyPage {
                center: style_export::leaf_chat_bar_center(style_export::LeafChatBarCenterArgs {
                    text: local_channel.res.memo_short.clone(),
                    link: Some(ministate_octothorpe(&Ministate::ChannelMenu(MinistateChannel {
                        channel: local_channel.res.id.clone(),
                        reset: reset_id.clone(),
                    }))),
                }).root,
                body: vec![inf.el()],
            }
        }
    });
}
