use {
    crate::{
        js::{
            el_async,
            style_export,
        },
        localdata::{
            req_api_channelgroups,
            req_api_channels,
        },
        state::{
            LocalCocg,
            Ministate,
            MinistateChannel,
            MinistateChannelGroup,
            ministate_octothorpe,
            state,
            merge_top,
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
};

fn build_root_children(pc: &mut ProcessingContext) -> El {
    let root = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    root.ref_own(|e| link!((pc = pc), (top = state().top.clone()), (), (e = e.weak()) {
        let e = e.upgrade()?;
        for change in &*top.borrow_changes() {
            let mut add = vec![];
            for cocg in &change.add {
                let el1 = match cocg {
                    LocalCocg::Channel(c) => style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: c.res.memo_short.clone(),
                        link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                            id: c.res.id.clone(),
                            own_identity: c.res.own_identity.clone(),
                            reset_id: None,
                        })),
                    }).root,
                    LocalCocg::ChannelGroup(cg) => {
                        let out = style_export::leaf_menu_group(style_export::LeafMenuGroupArgs {
                            text: cg.v.res.memo_short.clone(),
                            link: ministate_octothorpe(&Ministate::ChannelGroup(MinistateChannelGroup {
                                id: cg.v.res.id,
                                reset_id: None,
                            })),
                            children: vec![],
                        });
                        out
                            .group_el
                            .ref_own(|e| link!((_pc = pc), (children = cg.children.clone()), (), (e = e.weak()) {
                                let e = e.upgrade()?;
                                for change in &*children.borrow_changes() {
                                    let mut add = vec![];
                                    for c in &change.add {
                                        let child_e = style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                                            text: c.res.memo_short.clone(),
                                            link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                                                id: c.res.id.clone(),
                                                own_identity: c.res.own_identity.clone(),
                                                reset_id: None,
                                            })),
                                        }).root;
                                        add.push(child_e);
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

    // Assemble and return
    return out.root;
}
