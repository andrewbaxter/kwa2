use {
    crate::{
        api::channel_image,
        js::{
            el_async,
            style_export,
        },
        localdata::{
            req_api_channelgroups,
            req_api_channels,
        },
        state::{
            LocalChannel1,
            LocalCocg,
            Ministate,
            MinistateChannel,
            MinistateChannelGroup,
            merge_top,
            ministate_octothorpe,
            state,
        },
    },
    flowcontrol::ta_return,
    futures::join,
    lunk::{
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        spawn_rooted,
    },
    std::rc::Rc,
};

fn link_selected(pc: &mut ProcessingContext, el: &El, selected: fn(&Ministate) -> bool) {
    el.ref_own(
        |el_| link!((_pc = pc), (ministate = state().ministate.clone()), (), (el_ = el_.weak(), selected = selected) {
            let el_ = el_.upgrade()?;
            el_.ref_modify_classes(&[(&style_export::class_state_selected().value, selected(&*ministate.borrow()))]);
        }),
    );
}

fn build_channel_link(pc: &mut ProcessingContext, c: &Rc<LocalChannel1>) -> El {
    let img = channel_image(&c.id, &c.portrait.get());
    let child = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
        text: c.name.get(),
        link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
            id: c.id.clone(),
            own_identity: c.own_identity.clone(),
            reset_id: None,
        })),
        image: img.image,
        generated_image: img.generated_image,
    });
    child.text.ref_own(|el_| link!((_pc = pc), (name = c.name.clone()), (), (el_ = el_.weak()) {
        let el_ = el_.upgrade()?;
        el_.ref_text(&*name.borrow());
    }));
    child
        .image
        .unwrap()
        .ref_own(|el_| link!((_pc = pc), (portrait = c.portrait.clone()), (), (el_ = el_.weak(), id = c.id.clone()) {
            let el_ = el_.upgrade()?;
            let img = channel_image(&id, &*portrait.borrow());
            el_.ref_clear();
            el_.ref_push(style_export::leaf_menu_link_image(style_export::LeafMenuLinkImageArgs {
                image: img.image,
                generated_image: img.generated_image,
            }).root);
        }));
    child.unread.ref_own(|el_| link!((_pc = pc), (unread = c.unread.clone()), (), (el_ = el_.weak()) {
        let el_ = el_.upgrade()?;
        el_.ref_modify_classes(&[(&style_export::class_state_hidden().value, !*unread.borrow())]);
    }));
    child
        .root
        .ref_own(
            |el_| link!(
                (_pc = pc),
                (ministate = state().ministate.clone()),
                (),
                (el_ = el_.weak(), id = c.id.clone()) {
                    let el_ = el_.upgrade()?;
                    let selected = match &*ministate.borrow() {
                        Ministate::Channel(m) => m.id == *id,
                        _ => false,
                    };
                    el_.ref_modify_classes(&[(&style_export::class_state_selected().value, selected)]);
                }
            ),
        );
    return child.root;
}

fn build_root_children(pc: &mut ProcessingContext) -> El {
    let root = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    root.ref_own(|e| link!((pc = pc), (top = state().top.clone()), (), (e = e.weak()) {
        let e = e.upgrade()?;
        for change in &*top.borrow_changes() {
            let mut add = vec![];
            for cocg in &change.add {
                let el1 = match cocg {
                    LocalCocg::Channel(c) => build_channel_link(pc, c),
                    LocalCocg::ChannelGroup(cg) => {
                        let out = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                            text: cg.name.get(),
                            link: ministate_octothorpe(&Ministate::ChannelGroup(MinistateChannelGroup {
                                id: cg.id.clone(),
                                reset_id: None,
                            })),
                            image: None,
                            generated_image: None,
                            children: vec![],
                        });
                        out.text.ref_own(|el_| link!((_pc = pc), (name = cg.name.clone()), (), (el_ = el_.weak()) {
                            let el_ = el_.upgrade()?;
                            el_.ref_text(&*name.borrow());
                        }));
                        out
                            .unread
                            .ref_own(|el_| link!((_pc = pc), (unread = cg.unread.clone()), (), (el_ = el_.weak()) {
                                let el_ = el_.upgrade()?;
                                el_.ref_modify_classes(
                                    &[(&style_export::class_state_hidden().value, !*unread.borrow())]
                                );
                            }));
                        out
                            .link
                            .ref_own(
                                |el_| link!(
                                    (_pc = pc),
                                    (ministate = state().ministate.clone()),
                                    (),
                                    (el_ = el_.weak(), id = cg.id.clone()) {
                                        let el_ = el_.upgrade()?;
                                        let selected = match &*ministate.borrow() {
                                            Ministate::ChannelGroup(m) => m.id == *id,
                                            _ => false,
                                        };
                                        el_.ref_modify_classes(
                                            &[(&style_export::class_state_selected().value, selected)]
                                        );
                                    }
                                ),
                            );
                        out.group.ref_own(|e| link!((pc = pc), (children = cg.children.clone()), (), (e = e.weak()) {
                            let e = e.upgrade()?;
                            for change in &*children.borrow_changes() {
                                let mut add = vec![];
                                for c in &change.add {
                                    add.push(build_channel_link(pc, c));
                                }
                                e.ref_splice(change.offset, change.remove, add);
                            }
                        }));
                        out.root
                    },
                };
                add.push(el1);
            }
            e.ref_splice(change.offset, change.remove, add);
        }
    }));
    return root;
}

pub fn build(pc: &mut ProcessingContext) -> El {
    // Pull new elements in the background
    let bg_refresh = {
        let eg = pc.eg();
        async move {
            ta_return!((), String);
            let (cs, cgs) = join!(req_api_channels(None), req_api_channelgroups(None));
            let cs = cs?;
            let cgs = cgs?;
            eg.event(|pc| {
                merge_top(pc, cs, cgs);
            }).unwrap();
            return Ok(());
        }
    };
    let body;
    if state().top.borrow_values().is_empty() {
        body = el_async({
            let eg = pc.eg();
            async move {
                ta_return!(Vec < El >, String);
                bg_refresh.await?;
                return Ok(eg.event(|pc| {
                    return vec![build_root_children(pc)];
                }).unwrap());
            }
        });
    } else {
        let body1 = build_root_children(pc);
        body1.ref_own(move |_| spawn_rooted(async move {
            if let Err(e) = bg_refresh.await {
                state().log.log(&format!("Refreshing channels failed: {}", e));
            }
        }));
        body = body1;
    }

    // Other widgets, assemble and return
    let out = style_export::cont_page_top(style_export::ContPageTopArgs {
        identities_link: ministate_octothorpe(&Ministate::Identities),
        settings_link: ministate_octothorpe(&Ministate::Settings),
        add_link: ministate_octothorpe(&Ministate::TopAdd),
        body: vec![body],
    });
    link_selected(pc, &out.identities_link, |s| match s {
        Ministate::Identities |
        Ministate::IdentitiesNew |
        Ministate::Identity(_) |
        Ministate::IdentityEdit(_) |
        Ministate::IdentityDelete(_) |
        Ministate::IdentityInvites(_) |
        Ministate::IdentityInviteNew(_) |
        Ministate::IdentityInvite(_) |
        Ministate::IdentityInviteEdit(_) |
        Ministate::IdentityInviteDelete(_) => true,
        _ => false,
    });
    link_selected(pc, &out.settings_link, |s| match s {
        Ministate::Settings => true,
        _ => false,
    });
    link_selected(pc, &out.add_link, |s| match s {
        Ministate::TopAdd |
        Ministate::ChannelJoinUrl |
        Ministate::ChannelJoinQr |
        Ministate::ChannelJoinConfirm(_) |
        Ministate::ChannelNew |
        Ministate::ChannelGroupNew => true,
        _ => false,
    });

    // Assemble and return
    return out.root;
}
