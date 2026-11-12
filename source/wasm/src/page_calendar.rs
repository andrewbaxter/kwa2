use {
    crate::{
        calendar::{
            CalChannel,
            CalendarViewKey,
            DayAction,
            DayLayer,
            build_calendar,
            group_channels,
            remember_calendar_view,
        },
        js::style_export,
        localdata::req_api_channels,
        pageutil::build_nol_chat_bar,
        state::{
            Ministate,
            MinistateChannel,
            MinistateChannelGroup,
            MinistateChannelSub,
            get_or_req_channel,
            get_or_req_channelgroup,
            ministate_octothorpe,
            set_page,
            spawn_rooted_log,
            state,
        },
    },
    jiff::Zoned,
    lunk::{
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        scope_any,
    },
    shared::interface::shared::ChannelGroupId,
    std::rc::Rc,
};

fn assemble(
    pc: &mut ProcessingContext,
    channels: Vec<CalChannel>,
    head_bar: El,
    own: Vec<rooting::ScopeValue>,
) -> El {
    let stack = style_export::cont_calendar_stack().root;
    let (cal, inf) =
        build_calendar(pc, DayLayer::Page(stack.clone()), channels, DayAction::GoTo, Zoned::now().date());
    inf.padding_pre_el().ref_push(head_bar);
    stack.ref_push(inf.el());
    *state().current_calendar.borrow_mut() = Some(Rc::downgrade(&cal.0));
    stack.ref_own(|_| (cal, inf, own));
    return stack;
}

pub fn build_channel(pc: &mut ProcessingContext, s: &MinistateChannelSub) -> El {
    let key = CalendarViewKey::Channel(s.id.clone());
    remember_calendar_view(key.clone(), true);
    let messages_link =
        style_export::leaf_chat_head_bar_right_messages(
            style_export::LeafChatHeadBarRightMessagesArgs {
                link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                    id: s.id.clone(),
                    own_identity: s.own_identity.clone(),
                    reset_id: None,
                })),
            },
        ).root;
    messages_link.ref_on("click", move |_| remember_calendar_view(key.clone(), false));
    let head_bar = build_nol_chat_bar(pc, &Ministate::Top, get_or_req_channel(&pc.eg(), &s.id, true), messages_link, {
        let s = s.clone();
        move |pc, local_channel| {
            let center = style_export::leaf_chat_head_bar_center(style_export::LeafChatHeadBarCenterArgs {
                text: local_channel.name.get(),
                link: Some(ministate_octothorpe(&Ministate::ChannelMenu(s.clone()))),
            }).root;
            center.ref_own(|e| link!((_pc = pc), (name = local_channel.name.clone()), (), (e = e.weak()) {
                let e = e.upgrade()?;
                e.ref_text(&*name.borrow());
            }));
            return center;
        }
    });
    return assemble(pc, vec![CalChannel {
        id: s.id.clone(),
        own_identity: s.own_identity.clone(),
    }], head_bar.root, vec![]);
}

pub fn build_group(pc: &mut ProcessingContext, id: &ChannelGroupId) -> El {
    let key = CalendarViewKey::Group(*id);
    remember_calendar_view(key.clone(), true);
    let messages_link =
        style_export::leaf_chat_head_bar_right_messages(
            style_export::LeafChatHeadBarRightMessagesArgs {
                link: ministate_octothorpe(&Ministate::ChannelGroup(MinistateChannelGroup {
                    id: *id,
                    reset_id: None,
                })),
            },
        ).root;
    messages_link.ref_on("click", move |_| remember_calendar_view(key.clone(), false));
    let head_bar =
        build_nol_chat_bar(pc, &Ministate::Top, get_or_req_channelgroup(&pc.eg(), id, true), messages_link, {
            let id = *id;
            move |pc, local_channelgroup| {
                let center = style_export::leaf_chat_head_bar_center(style_export::LeafChatHeadBarCenterArgs {
                    text: local_channelgroup.name.get(),
                    link: Some(ministate_octothorpe(&Ministate::ChannelGroupMenu(id))),
                }).root;
                center.ref_own(|e| link!((_pc = pc), (name = local_channelgroup.name.clone()), (), (e = e.weak()) {
                    let e = e.upgrade()?;
                    e.ref_text(&*name.borrow());
                }));
                return center;
            }
        });
    let channels = group_channels(id);
    let refresh = spawn_rooted_log("Refreshing channel group calendar channels", {
        let id = *id;
        let eg = pc.eg();
        let initial = channels.clone();
        async move {
            req_api_channels(None).await?;
            if group_channels(&id) == initial {
                return Ok(());
            }
            if !matches!(&*state().ministate.borrow(), Ministate:: ChannelGroupCalendar(current) if * current == id) {
                return Ok(());
            }
            eg.event(|pc| set_page(build_group(pc, &id))).unwrap();
            return Ok(());
        }
    });
    return assemble(pc, channels, head_bar.root, vec![scope_any(refresh)]);
}
