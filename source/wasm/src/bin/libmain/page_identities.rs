use {
    super::api::req_post_json,
    crate::libmain::{
        page_top::LocalChannel,
        state::{
            ministate_octothorpe,
            state,
            Ministate,
            MinistateChannel,
        },
    },
    chrono::{
        DateTime,
        Utc,
    },
    flowcontrol::ta_return,
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    lunk::ProcessingContext,
    rooting::{
        spawn_rooted,
        El,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::wire::{
        c2s::{
            self,
            ChannelRes,
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    wasm::js::{
        el_async,
        style_export,
        LogJsErr,
    },
};

const LOCALSTORAGE_IDENTITIES: &str = "identities";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct LocalIdentity {
    id: Identity,
    memo_short: String,
    last_used: DateTime<Utc>,
    children: Vec<LocalChannel>,
}

pub fn build_page_identities(_pc: &mut ProcessingContext) -> El {
    let identity_elements = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let local_identities = match LocalStorage::get::<Vec<LocalIdentity>>(LOCALSTORAGE_IDENTITIES) {
        Ok(local_identities) => local_identities,
        Err(e) => {
            match e {
                gloo::storage::errors::StorageError::SerdeError(e) => {
                    state()
                        .log
                        .log(&format!("Error loading [{}] from local storage: {}", LOCALSTORAGE_IDENTITIES, e));
                },
                gloo::storage::errors::StorageError::KeyNotFound(_) => {
                    // nop
                },
                gloo::storage::errors::StorageError::JsError(e) => {
                    state()
                        .log
                        .log(&format!("Error loading [{}] from local storage: {}", LOCALSTORAGE_IDENTITIES, e));
                },
            }
            Default::default()
        },
    };
    let lookup_el_menu_identities = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_menu_identity_children = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_menu_channels = Rc::new(RefCell::new(HashMap::new()));

    // Build the immediately available options
    for local_identity in local_identities.clone() {
        let mut children = vec![];
        for local_channel in local_identity.children {
            let el2 = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: local_channel.memo_short,
                link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                    identity: local_channel.identity.clone(),
                    channel: local_channel.id.clone(),
                })),
            }).root;
            lookup_el_menu_channels.borrow_mut().insert((local_channel.identity, local_channel.id), el2.clone());
            children.push(el2);
        }
        let out = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
            text: local_identity.memo_short.clone(),
            link: ministate_octothorpe(&Ministate::Identity(local_identity.id)),
            children: children,
        });
        lookup_el_menu_identity_children.borrow_mut().insert(local_identity.id.clone(), out.group_el);
        lookup_el_menu_identities.borrow_mut().insert(local_identity.id, out.root.clone());
        identity_elements.ref_push(out.root);
    }

    // Pull new elements in the background
    let start_empty = local_identities.is_empty();
    let bg_refresh = {
        let local_identities = local_identities;
        let lookup_el_menu_identities = lookup_el_menu_identities.clone();
        let lookup_el_menu_identity_children = lookup_el_menu_identity_children.clone();
        async move {
            ta_return!(Vec < El >, String);
            let identities = req_post_json(&state().env.base_url, c2s::IdentityList).await?;
            let channels = req_post_json(&state().env.base_url, c2s::ChannelList).await?;
            let mut lookup_channels = HashMap::<Identity, Vec<ChannelRes>>::new();
            for channel in channels {
                lookup_channels.entry(channel.identity.clone()).or_default().push(channel);
            }

            // Diff level 1 identities
            let mut new_els1 = vec![];
            let mut changed1 = false;
            let mut new_local_identities = vec![];
            {
                let mut lookup_local_identities = HashMap::new();
                for local_identity in local_identities {
                    lookup_local_identities.insert(local_identity.id.clone(), local_identity);
                }
                for identity in identities {
                    let mut new_local_identity;
                    let new_group_children_el;
                    if let Some(local_identity) = lookup_local_identities.remove(&identity.id) {
                        new_local_identity = local_identity;
                        new_group_children_el =
                            lookup_el_menu_identity_children
                                .borrow_mut()
                                .remove(&identity.id)
                                .expect("Existing group but no corresponding element in lookup")
                                .clone();
                    } else {
                        changed1 = true;
                        new_local_identity = LocalIdentity {
                            id: identity.id,
                            memo_short: identity.memo_short.clone(),
                            last_used: Utc::now(),
                            children: vec![],
                        };
                        let next_el1 = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                            text: identity.memo_short,
                            link: ministate_octothorpe(&Ministate::Identity(identity.id)),
                            children: vec![],
                        });
                        new_els1.push(next_el1.root);
                        new_group_children_el = next_el1.group_el;
                    }

                    // Diff level 2 channels in this group
                    let mut new_out_els2 = vec![];
                    let mut changed2 = false;
                    let mut new_local_channels = vec![];
                    {
                        let mut lookup_local_channels = HashMap::new();
                        for local_channel in new_local_identity.children {
                            lookup_local_channels.insert(
                                (local_channel.identity.clone(), local_channel.id.clone()),
                                local_channel,
                            );
                        }
                        for channel in lookup_channels.remove(&identity.id).unwrap_or_default() {
                            let new_local_channel;
                            if let Some(local_channel) =
                                lookup_local_channels.remove(&(channel.identity.clone(), channel.id.clone())) {
                                new_local_channel = local_channel;
                            } else {
                                changed2 = true;
                                new_local_channel = LocalChannel {
                                    identity: channel.identity.clone(),
                                    id: channel.id.clone(),
                                    memo_short: channel.memo_short.clone(),
                                    last_used: Utc::now(),
                                };
                                new_out_els2.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                    text: channel.memo_short,
                                    link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                                        identity: channel.identity,
                                        channel: channel.id,
                                    })),
                                }).root);
                            }
                            new_local_channels.push(new_local_channel);
                        }
                        for (id, _) in lookup_local_channels {
                            let Some(channel_el) = lookup_el_menu_channels.borrow_mut().remove(&id) else {
                                continue;
                            };
                            channel_el.ref_replace(vec![]);
                            changed2 = true;
                        }
                    }

                    // Flush
                    new_group_children_el.ref_splice(0, 0, new_out_els2);
                    new_local_channels.sort_by_cached_key(|c| (c.last_used, c.memo_short.clone()));
                    new_local_identity.children = new_local_channels;
                    if changed2 {
                        new_local_identity.last_used = Utc::now();
                        changed1 = true;
                    }
                    new_local_identities.push(new_local_identity);
                }
                for (id, _) in lookup_local_identities {
                    let Some(channel_el) = lookup_el_menu_identities.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                    changed1 = true;
                }
            }
            if changed1 {
                new_local_identities.sort_by_cached_key(|c| (c.last_used, c.memo_short.clone()));
                LocalStorage::set(
                    LOCALSTORAGE_IDENTITIES,
                    new_local_identities,
                ).log(&state().log, "Error storing identities");
            }
            return Ok(new_els1);
        }
    };
    if start_empty {
        identity_elements.ref_push(el_async(bg_refresh));
    } else {
        identity_elements.ref_own(move |_| spawn_rooted(async move {
            if let Err(e) = bg_refresh.await {
                state().log.log(&format!("Refreshing channels failed: {}", e));
            }
        }));
    }

    // Other widgets, assemble and return
    let out = style_export::cont_page_top(style_export::ContPageTopArgs {
        identities_link: ministate_octothorpe(&Ministate::Identities),
        body: identity_elements,
    });

    // Assemble and return
    return out.root;
}
