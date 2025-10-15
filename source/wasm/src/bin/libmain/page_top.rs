use {
    super::api::req_post_json,
    crate::libmain::state::{
        ministate_octothorpe,
        state,
        Ministate,
    },
    chrono::{
        DateTime,
        Utc,
    },
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
    std::{
        cell::RefCell,
        collections::{
            HashMap,
        },
        rc::Rc,
    },
    wasm::{
        js::{
            el_async,
            style_export,
            LogJsErr,
        },
    },
};

const LOCALSTORAGE_CHANNELS: &str = "channels";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct LocalChannel {
    id: ChannelId,
    name: String,
    last_used: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct LocalChannelGroup {
    id: ChannelGroupId,
    name: String,
    last_used: DateTime<Utc>,
    children: Vec<LocalChannelOrGroup>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum LocalChannelOrGroup {
    Channel(LocalChannel),
    Group(LocalChannelGroup),
}

type ChannelId = String;
type ChannelGroupId = String;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct ApiChannel {
    id: ChannelId,
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct ApiGroup {
    id: ChannelGroupId,
    name: String,
    children: Vec<ApiChannelOrGroup>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum ApiChannelOrGroup {
    Channel(ApiChannel),
    Group(ApiGroup),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
enum ChannelOrGroupId {
    Channel(ChannelId),
    Group(ChannelGroupId),
}

pub fn build_page_top(pc: &mut ProcessingContext) -> El {
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
                    state().log.log_js(&format!("Error loading [{}] from local storage", LOCALSTORAGE_CHANNELS), e);
                },
            }
            Default::default()
        },
    };
    let lookup_menu_groups = Rc::new(RefCell::new(HashMap::new()));
    let lookup_menu_group_children = Rc::new(RefCell::new(HashMap::new()));
    let lookup_menu_channels = Rc::new(RefCell::new(HashMap::new()));

    // Pull new elements in the background
    let bg_refresh = {
        let local_cocgs = local_cocgs.clone();
        let lookup_menu_groups = lookup_menu_groups.clone();
        let lookup_menu_group_children = lookup_menu_group_children.clone();
        let lookup_menu_channels = lookup_menu_channels.clone();
        async move {
            let mut new_cocgs_root: Vec<ApiChannelOrGroup> =
                req_post_json(&state().env.base_url, ReqGetChannelTree).await?;
            let lookup_local_cocgs = local_cocgs.into_iter().map(|x| match &x {
                LocalChannelOrGroup::Channel(c) => {
                    (ChannelOrGroupId::Channel(c.id.clone()), x)
                },
                LocalChannelOrGroup::Group(g) => {
                    (ChannelOrGroupId::Channel(g.id.clone()), x)
                },
            }).collect::<HashMap<_, _>>();
            let mut changed = false;
            let mut new_local_cocgs = vec![];

            struct DiffRes {
                new_local_cocgs: Vec<LocalChannelOrGroup>,
                new_els: Vec<El>,
                changed: bool,
            }

            fn recursive_diff(
                lookup_el_menu_channels: &Rc<RefCell<HashMap<ChannelId, El>>>,
                lookup_el_menu_groups: &Rc<RefCell<HashMap<ChannelGroupId, El>>>,
                lookup_el_menu_group_children: &Rc<RefCell<HashMap<ChannelGroupId, El>>>,
                new_cocgs_root: Vec<ApiChannelOrGroup>,
                local_cocgs: Vec<LocalChannelOrGroup>,
            ) -> DiffRes {
                let mut out_els = vec![];
                let mut out_local_cocgs = vec![];
                let mut changed = false;
                let mut lookup_local_channels = HashMap::new();
                let mut lookup_local_channel_groups = HashMap::new();
                for cocg in local_cocgs {
                    match cocg {
                        LocalChannelOrGroup::Channel(c) => {
                            lookup_local_channels.insert(c.id.clone(), c);
                        },
                        LocalChannelOrGroup::Group(cg) => {
                            lookup_local_channel_groups.insert(cg.id.clone(), cg);
                        },
                    }
                }
                let mut new_local_cocgs = vec![];
                for cocg in new_cocgs_root {
                    let new_local_cocg;
                    match cocg {
                        ApiChannelOrGroup::Channel(c) => {
                            let new_local_channel;
                            if let Some(local_channel) = lookup_local_channels.remove(&c.id.clone()) {
                                new_local_channel = local_channel;
                            } else {
                                *changed = true;
                                new_local_channel = LocalChannel {
                                    id: c.id,
                                    name: c.name,
                                    last_used: Utc::now(),
                                };
                                out_els.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                    text: c.name,
                                    link: ministate_octothorpe(&Ministate::Channel(c.id)),
                                }).root);
                            }
                            new_local_cocg = LocalChannelOrGroup::Channel(new_local_channel);
                        },
                        ApiChannelOrGroup::Group(g) => {
                            let mut new_local_channel_group;
                            let new_group_children_el;
                            if let Some(local_channel_group) =
                                lookup_local_channel_groups.remove(&ChannelOrGroupId::Group(g.id)) {
                                new_local_channel_group = local_channel_group;
                                new_group_children_el =
                                    lookup_el_menu_group_children
                                        .remove(&g.id)
                                        .expect("Existing group but no corresponding element in lookup")
                                        .clone();
                            } else {
                                *changed = true;
                                new_local_channel_group = LocalChannelGroup {
                                    id: g.id,
                                    name: g.name,
                                    last_used: Utc::now(),
                                    children: vec![],
                                };
                                let next_el1 = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                                    text: g.name,
                                    link: ministate_octothorpe(&Ministate::ChannelGroup(g.id)),
                                    children: vec![],
                                }).root;
                                out_els.push(next_el1);
                                new_group_children_el = next_el1;
                            }
                            let mut child_res =
                                recursive_diff(
                                    lookup_el_menu_channels,
                                    lookup_el_menu_groups,
                                    lookup_el_menu_group_children,
                                    g.children,
                                    &mut new_local_channel_group.children,
                                );
                            new_group_children_el.ref_splice(0, 0, child_res.new_els);
                            child_res.new_local_cocgs.sort_by_cached_key(|c| match c {
                                LocalChannelOrGroup::Channel(c) => (c.last_used, c.name.clone()),
                                LocalChannelOrGroup::Group(cg) => (cg.last_used, cg.name.clone()),
                            });
                            new_local_channel_group.children = child_res.new_local_cocgs;
                            if child_res.changed {
                                new_local_channel_group.last_used = Utc::now();
                                changed = true;
                            }
                            new_local_cocg = LocalChannelOrGroup::Channel(new_local_channel_group);
                        },
                    }
                    new_local_cocgs.push(new_local_cocg);
                }
                for (id, _) in lookup_local_channels {
                    let Some(channel_el) = lookup_el_menu_channels.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                    changed = true;
                }
                for (id, _) in lookup_local_channel_groups {
                    let Some(channel_el) = lookup_el_menu_groups.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                    changed = true;
                }
                return DiffRes {
                    new_local_cocgs: new_local_cocgs,
                    new_els: out_els,
                    changed: changed,
                };
            }

            let mut child_res =
                recursive_diff(
                    lookup_menu_channels,
                    lookup_menu_groups,
                    lookup_menu_group_children,
                    new_cocgs_root,
                    local_cocgs,
                );
            if child_res.changed {
                child_res.new_local_cocgs.sort_by_cached_key(|c| match c {
                    LocalChannelOrGroup::Channel(c) => (c.last_used, c.name.clone()),
                    LocalChannelOrGroup::Group(cg) => (cg.last_used, cg.name.clone()),
                });
                new_local_cocgs.sort_by_cached_key(|c| c.name.clone());
                LocalStorage::set(
                    LOCALSTORAGE_CHANNELS,
                    child_res.new_local_cocgs,
                ).log(&state().log, "Error storing channels");
            }
        }
    };
    if local_cocgs.is_empty() {
        channel_elements.ref_push(el_async(bg_refresh));
    } else {
        channel_elements.ref_own(move |_| spawn_rooted(bg_refresh));
    }

    // Build the immediately available options
    for local_cocg in local_cocgs {
        fn recursive_build(
            lookup_el_menu_channels: &Rc<RefCell<HashMap<ChannelId, El>>>,
            lookup_el_menu_groups: &Rc<RefCell<HashMap<ChannelGroupId, El>>>,
            lookup_el_menu_group_children: &Rc<RefCell<HashMap<ChannelGroupId, El>>>,
            local_cocg: LocalChannelOrGroup,
        ) -> El {
            match local_cocg {
                LocalChannelOrGroup::Channel(c) => {
                    let out = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: c.name,
                        link: ministate_octothorpe(&Ministate::Channel(c.id.clone())),
                    }).root;
                    lookup_el_menu_channels.borrow_mut().insert(c.id, out.clone());
                    return out;
                },
                LocalChannelOrGroup::Group(cg) => {
                    let mut children = vec![];
                    for child in cg.children {
                        recursive_build(
                            lookup_el_menu_channels,
                            lookup_el_menu_groups,
                            lookup_el_menu_group_children,
                            child,
                        );
                    }
                    let out = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                        text: cg.name.clone(),
                        link: ministate_octothorpe(&Ministate::ChannelGroup(cg.name, cg.id)),
                        children: vec![],
                    });
                    out.group_el.ref_splice(0, 0, children);
                    lookup_el_menu_groups.borrow_mut().insert(cg.id.clone(), out.group_el);
                    lookup_el_menu_channels.borrow_mut().insert(cg.id, out.root.clone());
                    return out.root;
                },
            }
        }

        channel_elements.ref_push(
            recursive_build(lookup_menu_channels, lookup_menu_groups, lookup_menu_group_children, local_cocg),
        );
    }
    return style_export::cont_page_top(style_export::ContPageTopArgs {
        identities_link: ministate_octothorpe(&Ministate::Identities),
        body: channel_elements,
    }).root;
}
