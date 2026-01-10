use {
    crate::{
        api::{
            req_get,
            req_post_json,
        },
        opfs::{
            opfs_delete,
            opfs_list_dir,
            opfs_read_json,
        },
        outbox::{
            OPFS_FILENAME_MAIN,
            OutboxMessage,
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
    gloo::timers::callback::Interval,
    lunk::EventGraph,
    rooting::spawn_rooted,
    shared::interface::{
        shared::{
            ChannelId,
            MessageClientId,
            QualifiedChannelId,
        },
        wire::c2s::{
            self,
            ActivityOffset,
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::str::FromStr,
    web_sys::FileSystemDirectoryHandle,
};

pub fn trigger_push() {
    let state1 = state();
    let mut bg_sending = state1.bg_pushing.borrow_mut();
    if bg_sending.is_some() {
        return;
    }
    *bg_sending = Some(spawn_rooted({
        async move {
            loop {
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
                                    for (timestamp, message) in opfs_channel_dir_entries(&channel_dir).await {
                                        let message =
                                            opfs_read_json::<OutboxMessage>(&message, OPFS_FILENAME_MAIN).await?;
                                        let client_id = MessageClientId::from_timestamp(timestamp);
                                        req_post_json(&state().env.base_url, c2s::MessagePush {
                                            client_id: client_id.clone(),
                                            channel: channel.clone(),
                                            identity: ident.clone(),
                                            body: message.body,
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
                    break;
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
                handle_notification(&eg, res.into_iter().collect());
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
