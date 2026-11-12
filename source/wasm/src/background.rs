use {
    crate::{
        api::{
            req_get,
            req_post_json,
            req_upload_file,
        },
        opfs::{
            opfs_delete,
            opfs_list_dir,
            opfs_read_file,
            opfs_read_json,
        },
        outbox::{
            OPFS_FILENAME_MAIN,
            OutboxBlock,
            OutboxFile,
            OutboxMessage,
            OutboxMessageRel,
            OutboxMessageReplyTo,
            opfs_channel_dir_entries,
            opfs_outbox,
        },
        state::{
            Ministate,
            save_unread,
            spawn_rooted_log,
            state,
        },
    },
    flowcontrol::ta_return,
    futures::future::join_all,
    gloo::timers::{
        callback::Interval,
        future::TimeoutFuture,
    },
    lunk::EventGraph,
    rooting::spawn_rooted,
    shared::interface::{
        shared::{
            ChannelId,
            MessageBlock,
            MessageBlockFile,
            MessageClientId,
            MessageRel,
            QualifiedChannelId,
            QualifiedFileId,
        },
        wire::{
            c2s::{
                self,
                ActivityOffset,
            },
            s2c::OutboxChange,
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::str::FromStr,
    web_sys::FileSystemDirectoryHandle,
};

const PUSH_BACKOFF_INITIAL_MS: u32 = 1000;

pub fn handle_outbox_change(eg: &EventGraph, change: OutboxChange) {
    let state = state();
    let current_chat = state.current_chat.borrow();
    let Some(c) = &*current_chat else {
        return;
    };
    let cf = c.chat_state2.channel_lookup.borrow();
    let Some(f) = cf.get(&change.channel) else {
        return;
    };
    let outboxes = f.server_outboxes.borrow();
    let Some(o) = outboxes.get(&change.sender) else {
        return;
    };
    o.notify(eg.clone());
}

pub fn trigger_push() {
    let state1 = state();
    let mut bg_sending = state1.bg_pushing.borrow_mut();
    if bg_sending.is_some() {
        state1.bg_pushing_dirty.set(true);
        return;
    }
    *bg_sending = Some(spawn_rooted({
        async move {
            let mut backoff = PUSH_BACKOFF_INITIAL_MS;
            loop {
                state().bg_pushing_dirty.set(false);
                let mut bg = vec![];
                let outbox_dir = opfs_outbox().await;
                for (sender_ident, ident_dir) in opfs_list_dir(&outbox_dir).await {
                    let sender_ident = match Identity::from_str(&sender_ident) {
                        Ok(i) => i,
                        Err(e) => {
                            state()
                                .log
                                .log(
                                    &format!(
                                        "Error while sending messages; couldn't parse opfs sender identity directory name [{}]: {}",
                                        sender_ident,
                                        e
                                    ),
                                );
                            continue;
                        },
                    };
                    let sender_ident_dir = FileSystemDirectoryHandle::from(ident_dir);
                    for (dest_ident, dest_ident_dir) in opfs_list_dir(&sender_ident_dir).await {
                        let dest_ident = match Identity::from_str(&dest_ident) {
                            Ok(i) => i,
                            Err(e) => {
                                state()
                                    .log
                                    .log(
                                        &format!(
                                            "Error while sending messages; couldn't parse opfs dest identity directory name [{}]: {}",
                                            sender_ident,
                                            e
                                        ),
                                    );
                                continue;
                            },
                        };
                        let dest_ident_dir = FileSystemDirectoryHandle::from(dest_ident_dir);
                        for (channel, channel_dir) in opfs_list_dir(&dest_ident_dir).await {
                            let channel = match str::parse(&channel) {
                                Ok(i) => i,
                                Err(e) => {
                                    state()
                                        .log
                                        .log(
                                            &format!(
                                                "Error while sending messages; couldn't parse opfs ident [{}] channel directory name [{}]: {}",
                                                sender_ident,
                                                channel,
                                                e
                                            ),
                                        );
                                    continue;
                                },
                            };
                            let channel = QualifiedChannelId {
                                identity: dest_ident.clone(),
                                channel: ChannelId(channel),
                            };
                            let channel_dir = FileSystemDirectoryHandle::from(channel_dir);
                            bg.push(spawn_rooted({
                                let ident = sender_ident.clone();
                                async move {
                                    ta_return!((), String);
                                    for (timestamp, message_dir) in opfs_channel_dir_entries(&channel_dir).await {
                                        let message =
                                            opfs_read_json::<OutboxMessage>(
                                                &message_dir,
                                                OPFS_FILENAME_MAIN,
                                            ).await?;
                                        let mut blocks = vec![];
                                        for block in &message.blocks {
                                            match block {
                                                OutboxBlock::Text(t) => blocks.push(MessageBlock::Text(t.clone())),
                                                OutboxBlock::Event(e) => blocks.push(
                                                    MessageBlock::Event(e.clone()),
                                                ),
                                                OutboxBlock::Extension(e) => blocks.push(
                                                    MessageBlock::Extension(e.clone()),
                                                ),
                                                OutboxBlock::File(f) => {
                                                    let file = match &f.file {
                                                        OutboxFile::Remote(file) => file.clone(),
                                                        OutboxFile::Local(seg) => {
                                                            let file = opfs_read_file(&message_dir, seg).await?;
                                                            QualifiedFileId {
                                                                identity: channel.identity.clone(),
                                                                hash: req_upload_file(&file, &f.mime_type)
                                                                    .await?
                                                                    .hash,
                                                            }
                                                        },
                                                    };
                                                    blocks.push(MessageBlock::File(MessageBlockFile {
                                                        file: file,
                                                        mime_type: f.mime_type.clone(),
                                                        size: f.size,
                                                        name: f.name.clone(),
                                                    }));
                                                },
                                            }
                                        }
                                        let client_id = MessageClientId::from_timestamp(timestamp);
                                        let rel = match &message.rel {
                                            OutboxMessageRel::None => MessageRel::None,
                                            OutboxMessageRel::ReplyTo(
                                                OutboxMessageReplyTo::Channel(target),
                                            ) => MessageRel
                                            ::ReplyTo(
                                                target.message.clone(),
                                            ),
                                            OutboxMessageRel::ReplyTo(OutboxMessageReplyTo::Outbox(target)) => {
                                                let Some(found) = req_get(c2s::MessageIdByClientId {
                                                    channel: channel.clone(),
                                                    client_id: target.clone(),
                                                }).await? else {
                                                    return Err(
                                                        format!(
                                                            "Reply target outbox message [{:?}] not yet accepted in channel [{:?}]",
                                                            target,
                                                            channel
                                                        ),
                                                    );
                                                };
                                                MessageRel::ReplyTo(found)
                                            },
                                            OutboxMessageRel::EditOf(target) => MessageRel::EditOf(
                                                target.message.clone(),
                                            ),
                                        };
                                        req_post_json(&state().env.base_url, c2s::MessagePush {
                                            client_id: client_id.clone(),
                                            channel: channel.clone(),
                                            identity: ident.clone(),
                                            rel: rel,
                                            blocks: blocks,
                                        })
                                            .await
                                            .map_err(
                                                |e| format!(
                                                    "Error sending message with ident [{}] channel [{:?}] idem [{:?}]: {}",
                                                    ident,
                                                    channel,
                                                    client_id,
                                                    e
                                                ),
                                            )?;
                                        opfs_delete(&channel_dir, &client_id.0).await;
                                    }
                                    return Ok(());
                                }
                            }));
                        }
                    }
                }
                if join_all(bg).await.into_iter().all(|x| {
                    match x {
                        Ok(r) => match r {
                            Ok(_) => {
                                return true;
                            },
                            Err(e) => {
                                state().log.log(&e);
                                return false;
                            },
                        },
                        Err(_) => {
                            return true;
                        },
                    }
                }) {
                    backoff = PUSH_BACKOFF_INITIAL_MS;
                    if !state().bg_pushing_dirty.get() {
                        break;
                    }
                } else {
                    TimeoutFuture::new(backoff).await;
                    backoff = (backoff * 2).min(5 * 60 * 1000);
                }
            }
            *state().bg_pushing.borrow_mut() = None;
        }
    }));
}

pub fn handle_notification(eg: &EventGraph, notifications: Vec<(QualifiedChannelId, ActivityOffset)>) {
    let state = state();
    let current_chat = state.current_chat.borrow();
    let mut has_feed = false;
    if let Some(c) = &*current_chat {
        let cf = c.chat_state2.channel_lookup.borrow();
        for (channel, offset) in &notifications {
            if let Some(f) = cf.get(channel) {
                f.channel.notify(eg, *offset);
                has_feed = true;
            }
        }
    }
    if matches!(&*state.ministate.borrow(), Ministate::ChannelCalendar(..) | Ministate::ChannelGroupCalendar(..)) {
        if let Some(cal) = state.current_calendar.borrow().as_ref().and_then(|c| c.upgrade()) {
            let cal = crate::calendar::Calendar(cal);
            for (channel, offset) in &notifications {
                cal.notify(channel, *offset);
            }
        }
    }
    if !matches!(&*state.ministate.borrow(), Ministate::Channel(..) | Ministate::ChannelGroup(..)) || !has_feed {
        eg.event(|pc| {
            let mut unread_changed = false;
            for (channel, offset) in &notifications {
                let lookup_channels = state.lookup_channel.borrow();
                let Some(lc) = lookup_channels.get(channel) else {
                    continue;
                };
                if lc.last_offset.get().map(|o| o <= *offset).unwrap_or(false) {
                    continue;
                }
                lc.last_offset.set(Some(*offset));
                lc.unread.set(pc, true);
                unread_changed = true;
                if let Some(group) = &*lc.group.borrow() {
                    if let Some(group) = state.lookup_channelgroup.borrow().get(group) {
                        group.unread.set(pc, true);
                        state.unread_any.set(pc, true);
                    }
                } else {
                    state.unread_any.set(pc, true);
                }

                // Borrow weirdness
                ();
            }
            if unread_changed {
                save_unread();
            }
        }).unwrap();
    }
}

pub fn schedule_trigger_pull(eg: EventGraph) {
    fn trigger_pull(eg: &EventGraph) {
        *state().bg_pulling.borrow_mut() = Some(spawn_rooted_log("Pulling fresh messages", {
            let eg = eg.clone();
            async move {
                ta_return!((), String);
                let res = req_get(c2s::ActivityLatestAll {}).await?;
                handle_notification(&eg, res);
                return Ok(());
            }
        }));
    }

    trigger_pull(&eg);
    *state().bg_pulling_interval.borrow_mut() = Some(Interval::new(5 * 60 * 1000, {
        move || {
            trigger_pull(&eg);
        }
    }));
}
