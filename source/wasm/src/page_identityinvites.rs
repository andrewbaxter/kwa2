use {
    crate::{
        localdata::{
            get_stored_api_identityinvites,
            req_api_identityinvites,
        },
        state::{
            ministate_octothorpe,
            state,
            Ministate,
        },
    },
    flowcontrol::ta_return,
    lunk::ProcessingContext,
    rooting::{
        spawn_rooted,
        El,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    crate::js::{
        el_async,
        style_export,
    },
};

pub fn build(_pc: &mut ProcessingContext, identity: &Identity) -> El {
    let inv_elements = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let old_invites =
        get_stored_api_identityinvites(None)
            .into_iter()
            .filter(|x| x.res.token.identity == *identity)
            .collect::<Vec<_>>();
    let lookup_el_invites = Rc::new(RefCell::new(HashMap::new()));

    // Build the immediately available options
    for old_invite in old_invites.clone() {
        let out = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: old_invite.res.memo_short.clone(),
            link: ministate_octothorpe(&Ministate::IdentityInvite(old_invite.res.id)),
        });
        lookup_el_invites.borrow_mut().insert(old_invite.res.id, out.root.clone());
        inv_elements.ref_push(out.root);
    }

    // Pull new elements in the background
    let start_empty = old_invites.is_empty();
    let bg_refresh = {
        let old_invites1 = old_invites;
        let lookup_el_invites = lookup_el_invites.clone();
        let identity = identity.clone();
        async move {
            ta_return!(Vec < El >, String);
            let new_invites =
                req_api_identityinvites(None)
                    .await?
                    .into_iter()
                    .filter(|x| x.res.token.identity == identity)
                    .collect::<Vec<_>>();

            // Diff level 1 identities
            let mut new_els1 = vec![];
            {
                let mut old_invites = HashMap::new();
                for old_invite in old_invites1 {
                    old_invites.insert(old_invite.res.id, old_invite);
                }
                for new_invite in new_invites {
                    if let Some(_) = old_invites.remove(&new_invite.res.id) {
                        // nop
                    } else {
                        let next_el1 = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                            text: new_invite.res.memo_short,
                            link: ministate_octothorpe(&Ministate::IdentityInvite(new_invite.res.id)),
                        });
                        new_els1.push(next_el1.root);
                    }
                }
                for (id, _) in old_invites {
                    let Some(channel_el) = lookup_el_invites.borrow_mut().remove(&id) else {
                        continue;
                    };
                    channel_el.ref_replace(vec![]);
                }
            }
            return Ok(new_els1);
        }
    };
    if start_empty {
        inv_elements.ref_push(el_async(bg_refresh));
    } else {
        inv_elements.ref_own(move |_| spawn_rooted(async move {
            if let Err(e) = bg_refresh.await {
                state().log.log(&format!("Refreshing invites failed: {}", e));
            }
        }));
    }

    // Other widgets, assemble and return
    let out = style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        style_export::cont_menu_bar(style_export::ContMenuBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            text: format!("Invites"),
            center_link: None,
            right: Some(
                style_export::leaf_menu_bar_add(
                    style_export::LeafMenuBarAddArgs {
                        link: ministate_octothorpe(&Ministate::IdentityInviteNew(identity.clone())),
                    },
                ).root,
            ),
        }).root,
        inv_elements
    ] });

    // Assemble and return
    return out.root;
}
