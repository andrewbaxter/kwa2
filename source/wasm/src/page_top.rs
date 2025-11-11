use {
    crate::{
        js::{
            el_async,
            style_export,
        },
        localdata::{
            get_stored_api_channelgroups,
            get_stored_api_channels,
            req_api_channelgroups,
            req_api_channels,
            LocalChannel,
            LocalChannelGroup,
        },
        state::{
            ministate_octothorpe,
            state,
            Ministate,
            MinistateChannel,
            MinistateChannelGroup,
        },
    },
    flowcontrol::ta_return,
    futures::join,
    lunk::ProcessingContext,
    rooting::{
        spawn_rooted,
        El,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::shared::ChannelGroupId,
    std::{
        cell::RefCell,
        collections::{
            HashMap,
            HashSet,
        },
        rc::Rc,
    },
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct LocalChannelGroup1 {
    v: LocalChannelGroup,
    children: Vec<LocalChannel>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum LocalCocg {
    Channel(LocalChannel),
    ChannelGroup(LocalChannelGroup1),
}

fn combine_local_cocgs(
    local_channels: Vec<LocalChannel>,
    local_channelgroups: Vec<LocalChannelGroup>,
) -> Vec<LocalCocg> {
    let mut out = vec![];
    let mut lookup_children = HashMap::<ChannelGroupId, Vec<LocalChannel>>::new();
    for channel in local_channels {
        if let Some(g) = &channel.res.group {
            lookup_children.entry(*g).or_default().push(channel);
        } else {
            out.push(LocalCocg::Channel(channel));
        }
    }
    for group in local_channelgroups {
        let mut children = lookup_children.remove(&group.res.id).unwrap_or_default();
        children.sort_by_cached_key(|x| (std::cmp::Reverse(x.last_used), x.res.memo_short.clone()));
        out.push(LocalCocg::ChannelGroup(LocalChannelGroup1 {
            children: children,
            v: group,
        }));
    }
    out.sort_by_cached_key(|x| match x {
        LocalCocg::Channel(x) => (std::cmp::Reverse(x.last_used), x.res.memo_short.clone()),
        LocalCocg::ChannelGroup(x) => (std::cmp::Reverse(x.v.last_used), x.v.res.memo_short.clone()),
    });
    return out;
}

pub fn build(_pc: &mut ProcessingContext) -> El {
    let channel_elements = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let old_cocgs = combine_local_cocgs(get_stored_api_channels(None), get_stored_api_channelgroups(None));
    let lookup_el_menu_groups = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_menu_group_children = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_menu_channels = Rc::new(RefCell::new(HashMap::new()));

    // Build the immediately available options
    for old_cocg1 in old_cocgs.clone() {
        let el1 = match old_cocg1 {
            LocalCocg::Channel(old_c) => {
                let out = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: old_c.res.memo_short,
                    link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                        channel: old_c.res.id.clone(),
                        reset: None,
                    })),
                }).root;
                lookup_el_menu_channels.borrow_mut().insert(old_c.res.id, out.clone());
                out
            },
            LocalCocg::ChannelGroup(old_cg) => {
                let mut children = vec![];
                for local_c2 in old_cg.children {
                    let el2 = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: local_c2.res.memo_short,
                        link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                            channel: local_c2.res.id.clone(),
                            reset: None,
                        })),
                    }).root;
                    lookup_el_menu_channels.borrow_mut().insert(local_c2.res.id, el2.clone());
                    children.push(el2);
                }
                let out = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                    text: old_cg.v.res.memo_short.clone(),
                    link: ministate_octothorpe(&Ministate::ChannelGroup(MinistateChannelGroup {
                        channelgroup: old_cg.v.res.id,
                        reset: None,
                    })),
                    children: vec![],
                });
                out.group_el.ref_splice(0, 0, children);
                lookup_el_menu_group_children.borrow_mut().insert(old_cg.v.res.id.clone(), out.group_el);
                lookup_el_menu_groups.borrow_mut().insert(old_cg.v.res.id, out.root.clone());
                out.root
            },
        };
        channel_elements.ref_push(el1);
    }

    // Pull new elements in the background
    let start_empty = old_cocgs.is_empty();
    let bg_refresh = {
        let old_cocgs1 = old_cocgs;
        let lookup_el_menu_groups = lookup_el_menu_groups.clone();
        let lookup_el_menu_group_children = lookup_el_menu_group_children.clone();
        let lookup_el_menu_channels = lookup_el_menu_channels.clone();
        async move {
            ta_return!(Vec < El >, String);
            let new_cocgs1 = {
                let (c, cg) = join!(req_api_channels(None), req_api_channelgroups(None));
                combine_local_cocgs(c?, cg?)
            };

            // Diff level 1 channels/groups
            let mut new_els1 = vec![];
            {
                let mut old_cs = HashSet::new();
                let mut old_cgs = HashMap::new();
                for old_cocg1 in old_cocgs1 {
                    match old_cocg1 {
                        LocalCocg::Channel(c) => {
                            old_cs.insert(c.res.id);
                        },
                        LocalCocg::ChannelGroup(cg) => {
                            old_cgs.insert(cg.v.res.id, cg);
                        },
                    }
                }
                for cocg1 in new_cocgs1 {
                    match cocg1 {
                        LocalCocg::Channel(new_c) => {
                            if old_cs.remove(&new_c.res.id) {
                                // nop
                            } else {
                                new_els1.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                    text: new_c.res.memo_short,
                                    link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                                        channel: new_c.res.id,
                                        reset: None,
                                    })),
                                }).root);
                            }
                        },
                        LocalCocg::ChannelGroup(new_cg) => {
                            let new_children_el;
                            let old_cs2;
                            if let Some(old_cg) = old_cgs.remove(&new_cg.v.res.id) {
                                new_children_el =
                                    lookup_el_menu_group_children
                                        .borrow_mut()
                                        .remove(&new_cg.v.res.id)
                                        .expect("Existing group but no corresponding element in lookup")
                                        .clone();
                                old_cs2 = old_cg.children;
                            } else {
                                let next_el1 = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                                    text: new_cg.v.res.memo_short,
                                    link: ministate_octothorpe(&Ministate::ChannelGroup(MinistateChannelGroup {
                                        channelgroup: new_cg.v.res.id,
                                        reset: None,
                                    })),
                                    children: vec![],
                                });
                                new_els1.push(next_el1.root);
                                new_children_el = next_el1.group_el;
                                old_cs2 = vec![];
                            }

                            // Diff level 2 channels in this group
                            let mut new_out_els2 = vec![];
                            {
                                let mut lookup_old_cs2 = HashSet::new();
                                for old_c2 in old_cs2 {
                                    lookup_old_cs2.insert(old_c2.res.id.clone());
                                }
                                for new_c2 in new_cg.children {
                                    if lookup_old_cs2.remove(&new_c2.res.id) {
                                        // nop
                                    } else {
                                        new_out_els2.push(
                                            style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                                text: new_c2.res.memo_short,
                                                link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                                                    channel: new_c2.res.id,
                                                    reset: None,
                                                })),
                                            }).root,
                                        );
                                    }
                                }
                                for id in lookup_old_cs2 {
                                    let Some(channel_el) = lookup_el_menu_channels.borrow_mut().remove(&id) else {
                                        continue;
                                    };
                                    channel_el.ref_replace(vec![]);
                                }
                            }

                            // Flush
                            new_children_el.ref_splice(0, 0, new_out_els2);
                        },
                    }
                }
                for id in old_cs {
                    let Some(channel_el) = lookup_el_menu_channels.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                }
                for (id, _) in old_cgs {
                    let Some(channel_el) = lookup_el_menu_groups.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                }
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
        settings_link: ministate_octothorpe(&Ministate::Settings),
        add_link: ministate_octothorpe(&Ministate::TopAdd),
        body: channel_elements,
    });

    // Assemble and return
    return out.root;
}
