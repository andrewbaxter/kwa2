use {
    crate::{
        js::{
            el_async,
            style_export,
        },
        localdata::{
            LocalChannelMember,
            LocalContact,
            get_stored_api_channelmembers,
            get_stored_api_contacts,
            req_api_channelmembers,
            req_api_contacts,
        },
        state::{
            Ministate,
            MinistateChannelMember,
            ministate_octothorpe,
            state,
        },
    },
    flowcontrol::ta_return,
    rooting::{
        El,
        spawn_rooted,
    },
    shared::interface::shared::QualifiedChannelId,
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    tokio::join,
};

fn build_member(
    channel: &QualifiedChannelId,
    member: &LocalChannelMember,
    contacts: &HashMap<Identity, LocalContact>,
) -> El {
    let text;
    match contacts.get(&member.res) {
        Some(contact) => {
            text = contact.res.memo_short.clone();
        },
        None => {
            text = member.res.to_string();
        },
    }
    return style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
        text: text,
        link: ministate_octothorpe(&Ministate::ChannelMember(MinistateChannelMember {
            channel: channel.clone(),
            identity: member.res.clone(),
        })),
    }).root;
}

pub fn build(channel: &QualifiedChannelId) -> El {
    let contact_elements = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let old_members = get_stored_api_channelmembers(channel, None);
    let old_contacts =
        get_stored_api_contacts(None).into_iter().map(|x| (x.res.id.clone(), x)).collect::<HashMap<_, _>>();
    let lookup_el_members = Rc::new(RefCell::new(HashMap::new()));

    // Build the immediately available options
    for old_member in old_members.clone() {
        let out = build_member(channel, &old_member, &old_contacts);
        lookup_el_members.borrow_mut().insert(old_member.res, out.clone());
        contact_elements.ref_push(out);
    }

    // Pull new elements in the background
    let start_empty = old_members.is_empty();
    let bg_refresh = {
        let old_members1 = old_members;
        let lookup_el_members = lookup_el_members.clone();
        let channel = channel.clone();
        async move {
            ta_return!(Vec < El >, String);
            let new_members = req_api_channelmembers(&channel, None);
            let new_contacts = req_api_contacts(None);
            let (new_members, new_contacts) = join!(new_members, new_contacts);
            let new_members = new_members?;
            let new_contacts =
                new_contacts?.into_iter().map(|x| (x.res.id.clone(), x)).collect::<HashMap<_, _>>();

            // Diff
            let mut new_els1 = vec![];
            {
                let mut old_members = HashMap::new();
                for old_invite in old_members1 {
                    old_members.insert(old_invite.res, old_invite);
                }
                for new_member in new_members {
                    if let Some(_) = old_members.remove(&new_member.res) {
                        // nop
                    } else {
                        let out = build_member(&channel, &new_member, &new_contacts);
                        new_els1.push(out);
                    }
                }
                for (id, _) in old_members {
                    let Some(channel_el) = lookup_el_members.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                }
            }
            return Ok(new_els1);
        }
    };
    if start_empty {
        contact_elements.ref_push(el_async(bg_refresh));
    } else {
        contact_elements.ref_own(move |_| spawn_rooted(async move {
            if let Err(e) = bg_refresh.await {
                state().log.log(&format!("Refreshing channel members failed: {}", e));
            }
        }));
    }

    // Other widgets, assemble and return
    let out = style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        style_export::cont_head_bar(style_export::ContHeadBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            center: style_export::leaf_head_bar_center(style_export::LeafHeadBarCenterArgs {
                text: format!("Members"),
                link: None,
            }).root,
            right: Some(
                style_export::leaf_menu_head_bar_right_add(
                    style_export::LeafMenuHeadBarRightAddArgs {
                        link: ministate_octothorpe(&Ministate::ChannelInviteNew(channel.clone())),
                    },
                ).root,
            ),
        }).root,
        contact_elements
    ] });

    // Assemble and return
    return out.root;
}
