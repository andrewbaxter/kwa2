use {
    crate::{
        js::{
            el_async,
            style_export,
        },
        localdata::{
            get_stored_api_channels,
            get_stored_api_identities,
            req_api_channels,
            req_api_identities,
            LocalChannel,
            LocalIdentity,
        },
        state::{
            ministate_octothorpe,
            state,
            Ministate,
            MinistateChannel,
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
    spaghettinuum::interface::identity::Identity,
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
struct LocalIdentity1 {
    v: LocalIdentity,
    children: Vec<LocalChannel>,
}

fn combine_local_values(
    local_identities: Vec<LocalIdentity>,
    local_channels: Vec<LocalChannel>,
) -> Vec<LocalIdentity1> {
    let mut lookup_children = HashMap::<Identity, Vec<LocalChannel>>::new();
    for channel in local_channels {
        lookup_children.entry(channel.res.id.identity.clone()).or_default().push(channel);
    }
    let mut out = vec![];
    for identity in local_identities {
        let mut children = lookup_children.remove(&identity.res.id).unwrap_or_default();
        children.sort_by_cached_key(|x| (std::cmp::Reverse(x.last_used), x.res.memo_short.clone()));
        out.push(LocalIdentity1 {
            children: children,
            v: identity,
        });
    }
    out.sort_by_cached_key(|x| (std::cmp::Reverse(x.v.last_used), x.v.res.memo_short.clone()));
    return out;
}

pub fn build(_pc: &mut ProcessingContext) -> El {
    let identity_elements = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let old_identities = combine_local_values(get_stored_api_identities(None), get_stored_api_channels(None));
    let lookup_el_identities = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_identity_children = Rc::new(RefCell::new(HashMap::new()));
    let lookup_el_channels = Rc::new(RefCell::new(HashMap::new()));

    // Build the immediately available options
    for old_identity in old_identities.clone() {
        let mut children = vec![];
        for old_channel in old_identity.children {
            let channel_el = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: old_channel.res.memo_short,
                link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                    id: old_channel.res.id.clone(),
                    own_identity: old_identity.v.res.id.clone(),
                    reset_id: None,
                })),
            }).root;
            lookup_el_channels.borrow_mut().insert(old_channel.res.id, channel_el.clone());
            children.push(channel_el);
        }
        let out = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
            text: old_identity.v.res.memo_short.clone(),
            link: ministate_octothorpe(&Ministate::Identity(old_identity.v.res.id)),
            children: vec![],
        });
        out.group_el.ref_splice(0, 0, children);
        lookup_el_identity_children.borrow_mut().insert(old_identity.v.res.id.clone(), out.group_el);
        lookup_el_identities.borrow_mut().insert(old_identity.v.res.id, out.root.clone());
        identity_elements.ref_push(out.root);
    }

    // Pull new elements in the background
    let start_empty = old_identities.is_empty();
    let bg_refresh = {
        let old_identities1 = old_identities;
        let lookup_el_identities = lookup_el_identities.clone();
        let lookup_el_identity_children = lookup_el_identity_children.clone();
        let lookup_el_channels = lookup_el_channels.clone();
        async move {
            ta_return!(Vec < El >, String);
            let new_identities = {
                let (c, cg) = join!(req_api_identities(None), req_api_channels(None));
                combine_local_values(c?, cg?)
            };

            // Diff level 1 channels/groups
            let mut new_els1 = vec![];
            {
                let mut old_identities = HashMap::new();
                for old_identity in old_identities1 {
                    old_identities.insert(old_identity.v.res.id, old_identity);
                }
                for new_identity in new_identities {
                    let new_children_el;
                    let old_channels1;
                    if let Some(old_cg) = old_identities.remove(&new_identity.v.res.id) {
                        new_children_el =
                            lookup_el_identity_children
                                .borrow_mut()
                                .remove(&new_identity.v.res.id)
                                .expect("Existing group but no corresponding element in lookup")
                                .clone();
                        old_channels1 = old_cg.children;
                    } else {
                        let next_el1 = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                            text: new_identity.v.res.memo_short,
                            link: ministate_octothorpe(&Ministate::Identity(new_identity.v.res.id)),
                            children: vec![],
                        });
                        new_els1.push(next_el1.root);
                        new_children_el = next_el1.group_el;
                        old_channels1 = vec![];
                    }

                    // Diff level 2 channels in this group
                    let mut new_out_els2 = vec![];
                    {
                        let mut old_channels = HashSet::new();
                        for old_channel in old_channels1 {
                            old_channels.insert(old_channel.res.id.clone());
                        }
                        for new_channel in new_identity.children {
                            if old_channels.remove(&new_channel.res.id) {
                                // nop
                            } else {
                                new_out_els2.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                    text: new_channel.res.memo_short,
                                    link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                                        id: new_channel.res.id,
                                        own_identity: new_identity.v.res.id.clone(),
                                        reset_id: None,
                                    })),
                                }).root);
                            }
                        }
                        for id in old_channels {
                            let Some(channel_el) = lookup_el_channels.borrow_mut().remove(&id) else {
                                continue;
                            };
                            channel_el.ref_replace(vec![]);
                        }
                    }

                    // Flush
                    new_children_el.ref_splice(0, 0, new_out_els2);
                }
                for (id, _) in old_identities {
                    let Some(channel_el) = lookup_el_identities.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                }
            }
            return Ok(new_els1);
        }
    };
    if start_empty {
        identity_elements.ref_push(el_async(bg_refresh));
    } else {
        identity_elements.ref_own(move |_| spawn_rooted(async move {
            if let Err(e) = bg_refresh.await {
                state().log.log(&format!("Refreshing identities failed: {}", e));
            }
        }));
    }

    // Other widgets, assemble and return
    let out = style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        style_export::cont_menu_bar(style_export::ContMenuBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            center: style_export::leaf_menu_bar_center(style_export::LeafMenuBarCenterArgs {
                text: format!("Identities"),
                link: None,
            }).root,
            right: Some(
                style_export::leaf_menu_bar_add(
                    style_export::LeafMenuBarAddArgs { link: ministate_octothorpe(&Ministate::IdentitiesNew) },
                ).root,
            ),
        }).root,
        identity_elements
    ] });

    // Assemble and return
    return out.root;
}
