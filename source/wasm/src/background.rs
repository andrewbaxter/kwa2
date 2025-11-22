use {
    crate::{
        api::{
            req_get,
            req_post_json,
        },
        outbox::{
            OPFS_FILENAME_MAIN,
            OutboxMessage,
            opfs_channel_dir_entries,
            opfs_delete,
            opfs_list_dir,
            opfs_outbox,
            opfs_read_json,
        },
        state::{
            spawn_rooted_log,
            state,
        },
    },
    flowcontrol::ta_return,
    futures::future::join_all,
    gloo::timers::callback::Interval,
    lunk::{
        EventGraph,
    },
    rooting::spawn_rooted,
    shared::interface::{
        shared::{
            ChannelId,
            MessageClientId,
            QualifiedChannelId,
        },
        wire::c2s,
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

pub fn schedule_trigger_pull(eg: EventGraph) {
    fn trigger_pull(eg: &EventGraph) {
        *state().bg_pulling.borrow_mut() = Some(spawn_rooted_log("Pulling fresh messages", {
            let eg = eg.clone();
            async move {
                ta_return!((), String);
                let res = req_get(&state().env.base_url, c2s::ActivityLatestAll {}).await?;
                let Some(chat) = state().current_chat.borrow().clone() else {
                    return Ok(());
                };
                let channel_lookup = chat.chat_state2.channel_lookup.borrow();
                for (channel, offset) in res {
                    let Some(feed) = channel_lookup.get(&channel) else {
                        continue;
                    };
                    feed.channel.notify(&eg, offset);
                }
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
