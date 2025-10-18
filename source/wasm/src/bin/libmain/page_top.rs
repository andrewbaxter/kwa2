use {
    super::api::req_post_json,
    crate::libmain::state::{
        ministate_octothorpe,
        state,
        Ministate,
        MinistateChannel,
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
            ChannelOrChannelGroup,
        },
        shared::{
            ChannelId,
            InternalChannelGroupId,
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

const LOCALSTORAGE_CHANNELS: &str = "channels";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LocalChannel {
    pub identity: Identity,
    pub id: ChannelId,
    pub memo_short: String,
    pub last_used: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct LocalChannelGroup {
    id: InternalChannelGroupId,
    memo_short: String,
    last_used: DateTime<Utc>,
    children: Vec<LocalChannel>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum LocalChannelOrGroup {
    Channel(LocalChannel),
    Group(LocalChannelGroup),
}

pub fn build_page_top(_pc: &mut ProcessingContext) -> El {
    let channel_elements = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let local_cocgs = match LocalStorage::get::<Vec<LocalChannelOrGroup>>(LOCALSTORAGE_CHANNELS) {
        Ok(local_channels) => local_channels,
        Err(e) => {
            match e {
                gloo::storage::errors::StorageError::SerdeError(e) => {
                    state().log.log(&format!("Error loading [{}] from local storage: {}", LOCALSTORAGE_CHANNELS, e));
                },
                gloo::storage::errors::StorageError::KeyNotFound(_) => {
                    // nop
                },
                gloo::storage::errors::StorageError::JsError(e) => {
                    state().log.log(&format!("Error loading [{}] from local storage: {}", LOCALSTORAGE_CHANNELS, e));
                },
            }
            Default::default()
        },
    };
    let lookup_el_menu_groups = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_menu_group_children = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_menu_channels = Rc::new(RefCell::new(HashMap::new()));

    // Build the immediately available options
    for local_cocg1 in local_cocgs.clone() {
        let el1 = match local_cocg1 {
            LocalChannelOrGroup::Channel(c) => {
                let out = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: c.memo_short,
                    link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                        identity: c.identity.clone(),
                        channel: c.id.clone(),
                    })),
                }).root;
                lookup_el_menu_channels.borrow_mut().insert((c.identity.clone(), c.id), out.clone());
                out
            },
            LocalChannelOrGroup::Group(cg) => {
                let mut children = vec![];
                for local_c2 in cg.children {
                    let el2 = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: local_c2.memo_short,
                        link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                            identity: local_c2.identity.clone(),
                            channel: local_c2.id.clone(),
                        })),
                    }).root;
                    lookup_el_menu_channels.borrow_mut().insert((local_c2.identity, local_c2.id), el2.clone());
                    children.push(el2);
                }
                let out = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                    text: cg.memo_short.clone(),
                    link: ministate_octothorpe(&Ministate::ChannelGroup(cg.id)),
                    children: vec![],
                });
                out.group_el.ref_splice(0, 0, children);
                lookup_el_menu_group_children.borrow_mut().insert(cg.id.clone(), out.group_el);
                lookup_el_menu_groups.borrow_mut().insert(cg.id, out.root.clone());
                out.root
            },
        };
        channel_elements.ref_push(el1);
    }

    // Pull new elements in the background
    let start_empty = local_cocgs.is_empty();
    let bg_refresh = {
        let local_cocgs1 = local_cocgs;
        let lookup_el_menu_groups = lookup_el_menu_groups.clone();
        let lookup_el_menu_group_children = lookup_el_menu_group_children.clone();
        let lookup_el_menu_channels = lookup_el_menu_channels.clone();
        async move {
            ta_return!(Vec < El >, String);
            let cocgs1 = req_post_json(&state().env.base_url, c2s::ChannelOrChannelGroupTree).await?;

            // Diff level 1 channels/groups
            let mut new_els1 = vec![];
            let mut changed1 = false;
            let mut new_local_cocgs1 = vec![];
            {
                let mut lookup_local_channels1 = HashMap::new();
                let mut lookup_local_channel_groups1 = HashMap::new();
                for local_cocg1 in local_cocgs1 {
                    match local_cocg1 {
                        LocalChannelOrGroup::Channel(c) => {
                            lookup_local_channels1.insert((c.identity.clone(), c.id.clone()), c);
                        },
                        LocalChannelOrGroup::Group(cg) => {
                            lookup_local_channel_groups1.insert(cg.id.clone(), cg);
                        },
                    }
                }
                for cocg1 in cocgs1 {
                    let new_local_cocg;
                    match cocg1 {
                        ChannelOrChannelGroup::Channel(c) => {
                            let new_local_channel;
                            if let Some(local_channel) =
                                lookup_local_channels1.remove(&(c.identity.clone(), c.id.clone())) {
                                new_local_channel = local_channel;
                            } else {
                                changed1 = true;
                                new_local_channel = LocalChannel {
                                    identity: c.identity.clone(),
                                    id: c.id.clone(),
                                    memo_short: c.memo_short.clone(),
                                    last_used: Utc::now(),
                                };
                                new_els1.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                    text: c.memo_short,
                                    link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                                        identity: c.identity,
                                        channel: c.id,
                                    })),
                                }).root);
                            }
                            new_local_cocg = LocalChannelOrGroup::Channel(new_local_channel);
                        },
                        ChannelOrChannelGroup::ChannelGroup(g) => {
                            let mut new_local_channel_group;
                            let new_group_children_el;
                            if let Some(local_channel_group) =
                                lookup_local_channel_groups1.remove(&g.group.internal_id) {
                                new_local_channel_group = local_channel_group;
                                new_group_children_el =
                                    lookup_el_menu_group_children
                                        .borrow_mut()
                                        .remove(&g.group.internal_id)
                                        .expect("Existing group but no corresponding element in lookup")
                                        .clone();
                            } else {
                                changed1 = true;
                                new_local_channel_group = LocalChannelGroup {
                                    id: g.group.internal_id,
                                    memo_short: g.group.memo_short.clone(),
                                    last_used: Utc::now(),
                                    children: vec![],
                                };
                                let next_el1 = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                                    text: g.group.memo_short,
                                    link: ministate_octothorpe(&Ministate::ChannelGroup(g.group.internal_id)),
                                    children: vec![],
                                });
                                new_els1.push(next_el1.root);
                                new_group_children_el = next_el1.group_el;
                            }

                            // Diff level 2 channels in this group
                            let mut new_out_els2 = vec![];
                            let mut changed2 = false;
                            let mut new_local_cocgs2 = vec![];
                            {
                                let mut lookup_local_channels2 = HashMap::new();
                                for local_c2 in new_local_channel_group.children {
                                    lookup_local_channels2.insert(
                                        (local_c2.identity.clone(), local_c2.id.clone()),
                                        local_c2,
                                    );
                                }
                                for c2 in g.children {
                                    let new_local_channel;
                                    if let Some(local_channel) =
                                        lookup_local_channels2.remove(&(c2.identity.clone(), c2.id.clone())) {
                                        new_local_channel = local_channel;
                                    } else {
                                        changed2 = true;
                                        new_local_channel = LocalChannel {
                                            identity: c2.identity.clone(),
                                            id: c2.id.clone(),
                                            memo_short: c2.memo_short.clone(),
                                            last_used: Utc::now(),
                                        };
                                        new_out_els2.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                            text: c2.memo_short,
                                            link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                                                identity: c2.identity,
                                                channel: c2.id,
                                            })),
                                        }).root);
                                    }
                                    new_local_cocgs2.push(new_local_channel);
                                }
                                for (id, _) in lookup_local_channels2 {
                                    let Some(channel_el) = lookup_el_menu_channels.borrow_mut().remove(&id) else {
                                        continue;
                                    };
                                    channel_el.ref_replace(vec![]);
                                    changed2 = true;
                                }
                            }

                            // Flush
                            new_group_children_el.ref_splice(0, 0, new_out_els2);
                            new_local_cocgs2.sort_by_cached_key(|c| (c.last_used, c.memo_short.clone()));
                            new_local_channel_group.children = new_local_cocgs2;
                            if changed2 {
                                new_local_channel_group.last_used = Utc::now();
                                changed1 = true;
                            }
                            new_local_cocg = LocalChannelOrGroup::Group(new_local_channel_group);
                        },
                    }
                    new_local_cocgs1.push(new_local_cocg);
                }
                for (id, _) in lookup_local_channels1 {
                    let Some(channel_el) = lookup_el_menu_channels.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                    changed1 = true;
                }
                for (id, _) in lookup_local_channel_groups1 {
                    let Some(channel_el) = lookup_el_menu_groups.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                    changed1 = true;
                }
            }
            if changed1 {
                new_local_cocgs1.sort_by_cached_key(|c| match c {
                    LocalChannelOrGroup::Channel(c) => (c.last_used, c.memo_short.clone()),
                    LocalChannelOrGroup::Group(cg) => (cg.last_used, cg.memo_short.clone()),
                });
                LocalStorage::set(
                    LOCALSTORAGE_CHANNELS,
                    new_local_cocgs1,
                ).log(&state().log, "Error storing channels");
            }
            return Ok(new_els1);
        }
    };
    if start_empty {
        channel_elements.ref_push(el_async(bg_refresh));
    } else {
        channel_elements.ref_own(move |_| spawn_rooted(async move {
            if let Err(e) = bg_refresh.await {
                state().log.log(&format!("Refreshing channels failed: {}", e));
            }
        }));
    }

    // Other widgets, assemble and return
    let out = style_export::cont_page_top(style_export::ContPageTopArgs {
        identities_link: ministate_octothorpe(&Ministate::Identities),
        body: channel_elements,
    });

    // Assemble and return
    return out.root;
}
