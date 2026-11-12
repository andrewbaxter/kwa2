use {
    crate::{
        api::{
            identity_image,
            req_get,
        },
        blocks::view_blocks,
        calendar::{
            format_date,
            local_tz,
        },
        chat_message::build_message_body,
        js::{
            el_async,
            style_export,
        },
        localdata,
        state::{
            goto_replace_ministate,
            identity_portrait,
            ministate_octothorpe,
            state,
            Ministate,
            MinistateChannel,
            MinistateContact,
        },
    },
    flowcontrol::ta_return,
    jiff::civil::Date,
    lunk::{
        EventGraph,
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        WeakEl,
    },
    shared::interface::wire::c2s::{
        self,
        ContactPost,
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
};

struct Posts {
    eg: EventGraph,
    identity: spaghettinuum::interface::identity::Identity,
    left: bool,
    selected: RefCell<Option<(ContactPost, WeakEl)>>,
    last_date: RefCell<Option<Date>>,
}

fn build_page(posts: Rc<Posts>, page: usize) -> El {
    return el_async(async move {
        ta_return!(Vec < El >, String);
        let res = req_get(c2s::GetContactPosts {
            identity: posts.identity.clone(),
            page: page,
        }).await?;
        let mut out = vec![];
        if page == 0 && res.posts.is_empty() {
            out.push(
                style_export::leaf_calendar_day_empty(
                    style_export::LeafCalendarDayEmptyArgs { text: format!("No posts") },
                ).root,
            );
            return Ok(out);
        }
        let tz = local_tz();
        posts.eg.event(|pc| {
            for post in res.posts {
                let date = post.original_receive_time.to_zoned(tz.clone()).date();
                if *posts.last_date.borrow() != Some(date) {
                    *posts.last_date.borrow_mut() = Some(date);
                    out.push(
                        style_export::leaf_list_heading(
                            style_export::LeafListHeadingArgs { text: format_date(date) },
                        ).root,
                    );
                }
                let m_el = style_export::cont_chat_entry_mode_message(style_export::ContChatEntryModeMessageArgs {
                    left: posts.left,
                    date: post.original_receive_time.to_string(),
                });
                let portrait = identity_portrait(pc, &posts.identity);
                m_el
                    .portrait
                    .ref_own(
                        |portrait_el| link!(
                            (_pc = pc),
                            (portrait = portrait),
                            (),
                            (portrait_el = portrait_el.weak(), sender = posts.identity.clone()) {
                                let portrait_el = portrait_el.upgrade()?;
                                let img = identity_image(&sender, &*portrait.borrow());
                                portrait_el.ref_clear();
                                portrait_el.ref_push(
                                    style_export::leaf_chat_entry_portrait(style_export::LeafChatEntryPortraitArgs {
                                        image: img.image,
                                        generated_image: img.generated_image,
                                    },).root,
                                );
                            }
                        ),
                    );
                build_message_body(&m_el.body, &view_blocks(&post.original_id.channel, post.message.blocks.clone()));
                m_el.root.ref_on("click", {
                    let posts = posts.clone();
                    let el = m_el.root.weak();
                    move |_| {
                        let Some(el) = el.upgrade() else {
                            return;
                        };
                        let selected_class = style_export::class_state_selected().value;
                        if let Some((_, old)) = posts.selected.borrow_mut().take() {
                            if let Some(old) = old.upgrade() {
                                old.ref_modify_classes(&[(&selected_class, false)]);
                            }
                        }
                        el.ref_modify_classes(&[(&selected_class, true)]);
                        *posts.selected.borrow_mut() = Some((post.clone(), el.weak()));
                    }
                });
                out.push(m_el.root);
            }
        }).unwrap();
        if !res.last {
            let more =
                style_export::leaf_menu_button(style_export::LeafMenuButtonArgs { text: format!("More") }).root;
            more.ref_on("click", {
                let posts = posts.clone();
                let more = more.weak();
                move |_| {
                    let Some(more) = more.upgrade() else {
                        return;
                    };
                    more.ref_replace(vec![build_page(posts.clone(), page + 1)]);
                }
            });
            out.push(more);
        }
        return Ok(out);
    });
}

pub fn build(pc: &mut ProcessingContext, s: &MinistateContact) -> El {
    let posts = Rc::new(Posts {
        eg: pc.eg(),
        identity: s.identity.clone(),
        left: !localdata::get_stored_api_identities(None).into_iter().any(|x| x.res.id == s.identity),
        selected: RefCell::new(None),
        last_date: RefCell::new(None),
    });
    let head = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&Ministate::Contact(s.clone())),
        center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
            text: format!("Posts"),
            link: None,
        }).root,
        right: None,
    });
    head.back_unread.ref_own(|el_| link!((_pc = pc), (unread = state().unread_any.clone()), (), (el_ = el_.weak()), {
        let el_ = el_.upgrade()?;
        el_.ref_modify_classes(&[(&style_export::class_state_hidden().value, !*unread.borrow())]);
    }));
    let go_to = style_export::leaf_calendar_day_button_go_to().root;
    go_to.ref_on("click", {
        let posts = posts.clone();
        move |_| {
            let Some((post, _)) = posts.selected.borrow().clone() else {
                return;
            };
            posts.eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Channel(MinistateChannel {
                    id: post.original_id.channel.clone(),
                    own_identity: post.own_identity.clone(),
                    reset_id: Some(post.original_id.clone()),
                }));
            }).unwrap();
        }
    });
    return style_export::cont_calendar_page(style_export::ContCalendarPageArgs {
        head_bar: head.root,
        body: style_export::cont_calendar_day_list(
            style_export::ContCalendarDayListArgs { children: vec![build_page(posts.clone(), 0)] },
        ).root,
        controls: Some(
            style_export::cont_chat_controls_bar_mode_menu(
                style_export::ContChatControlsBarModeMenuArgs { children: vec![go_to] },
            ).root,
        ),
    }).root;
}
