use {
    crate::{
        api::{
            identity_image,
            req_get_retry,
        },
        blocks::view_blocks,
        chat_message::build_message_body,
        infinite::{
            Entry,
            Feed,
            Infinite,
            InfiniteEls,
            WeakInfinite,
        },
        js::style_export,
        modal::{
            ModalHandle,
            remove_child,
        },
        localdata::{
            self,
            get_stored_api_channels,
        },
        state::{
            Ministate,
            MinistateChannel,
            ministate_octothorpe,
            get_or_req_channel,
            goto_replace_ministate,
            identity_portrait,
            spawn_rooted_log,
            state,
        },
    },
    futures::channel::oneshot,
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    jiff::{
        ToSpan,
        Timestamp,
        Zoned,
        civil::{
            Date,
            Weekday,
        },
        tz::TimeZone,
    },
    lunk::{
        EventGraph,
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        WeakEl,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::{
        calendar::{
            CalendarWindow,
            Occurrence,
            event_occurrences,
            month_start,
            weekday_to_jiff,
        },
        interface::{
            shared::{
                EventWeekday,
                MessageBlock,
                MessageBody,
                MessageId,
                MessageRel,
                QualifiedChannelId,
                QualifiedMessageId,
            },
            wire::c2s::{
                self,
                ActivityOffset,
                ActivityPage,
                ActivityPageOffsetPos,
                CalendarMonth,
                CalendarPageRes,
                PagePosition,
            },
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::{
            BTreeSet,
            HashMap,
        },
        rc::{
            Rc,
            Weak,
        },
    },
    wasm_bindgen_futures::spawn_local,
};

pub const LOCALSTORAGE_WEEK_START: &str = "week_start";
pub const LOCALSTORAGE_CALENDAR_VIEWS: &str = "calendar_views";

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CalendarViewKey {
    Channel(QualifiedChannelId),
    Group(shared::interface::shared::ChannelGroupId),
}

pub fn calendar_view_remembered(key: &CalendarViewKey) -> bool {
    return LocalStorage::get::<BTreeSet<CalendarViewKey>>(LOCALSTORAGE_CALENDAR_VIEWS)
        .unwrap_or_default()
        .contains(key);
}

pub fn remember_calendar_view(key: CalendarViewKey, calendar: bool) {
    let mut views =
        LocalStorage::get::<BTreeSet<CalendarViewKey>>(LOCALSTORAGE_CALENDAR_VIEWS).unwrap_or_default();
    if calendar {
        views.insert(key);
    } else {
        views.remove(&key);
    }
    if let Err(e) = LocalStorage::set(LOCALSTORAGE_CALENDAR_VIEWS, views) {
        state().log.log(&format!("Error storing calendar view preference: {}", e));
    }
}

pub fn week_start() -> Weekday {
    match LocalStorage::get::<EventWeekday>(LOCALSTORAGE_WEEK_START) {
        Ok(v) => return weekday_to_jiff(v),
        Err(_) => return crate::js::locale_week_start(),
    }
}

pub fn local_tz() -> TimeZone {
    return TimeZone::system();
}

pub fn format_date(d: Date) -> String {
    return crate::js::intl_date(
        d,
        &[("weekday", "short"), ("day", "numeric"), ("month", "short"), ("year", "numeric")],
    );
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CalChannel {
    pub id: QualifiedChannelId,
    pub own_identity: Identity,
}

#[derive(Clone)]
pub struct CalItem {
    pub channel: CalChannel,
    pub root: QualifiedMessageId,
    pub blocks: Vec<MessageBlock>,
}

struct MonthChannel {
    have: Option<ActivityPageOffsetPos>,
    items: HashMap<MessageId, CalItem>,
}

struct CalMonth_ {
    month: CalendarMonth,
    el: El,
    cal: Weak<Calendar_>,
    channels: RefCell<HashMap<QualifiedChannelId, MonthChannel>>,
    loads: RefCell<Vec<oneshot::Receiver<()>>>,
}

type CalMonth = Rc<CalMonth_>;

pub enum DayLayer {
    Page(El),
    Modal,
}

#[derive(Clone)]
pub enum DayAction {
    GoTo,
    Select(Rc<dyn Fn(Date)>),
}

pub struct Calendar_ {
    eg: EventGraph,
    channels: Vec<CalChannel>,
    tz: TimeZone,
    today: Date,
    week_start: Weekday,
    layer: DayLayer,
    action: DayAction,
    months: RefCell<HashMap<CalendarMonth, Weak<CalMonth_>>>,
    activity: RefCell<HashMap<QualifiedChannelId, oneshot::Receiver<()>>>,
}

#[derive(Clone)]
pub struct Calendar(pub Rc<Calendar_>);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MonthIndex(i64);

impl MonthIndex {
    fn of(d: Date) -> MonthIndex {
        return MonthIndex(d.year() as i64 * 12 + d.month() as i64 - 1);
    }

    fn month(&self) -> CalendarMonth {
        return CalendarMonth {
            year: self.0.div_euclid(12) as i16,
            month: (self.0.rem_euclid(12) + 1) as i8,
        };
    }
}

impl Default for MonthIndex {
    fn default() -> Self {
        return MonthIndex::of(Zoned::now().date());
    }
}

pub struct MonthEntry {
    cal: Weak<Calendar_>,
    index: MonthIndex,
}

impl Entry for MonthEntry {
    type FeedId = ();
    type Time = MonthIndex;

    fn create_el(&self, pc: &mut ProcessingContext) -> El {
        let Some(cal) = self.cal.upgrade() else {
            return style_export::cont_page_blank().root;
        };
        let month = self.index.month();
        let existing = cal.months.borrow().get(&month).and_then(|m| m.upgrade());
        let m = match existing {
            Some(m) => m,
            None => {
                let m = Rc::new(CalMonth_ {
                    month: month,
                    el: style_export::cont_page_blank().root,
                    cal: Rc::downgrade(&cal),
                    channels: RefCell::new(HashMap::new()),
                    loads: RefCell::new(vec![]),
                });
                cal.months.borrow_mut().insert(month, Rc::downgrade(&m));
                render_month(pc, &cal, &m);
                for channel in &cal.channels {
                    m.loads.borrow_mut().push(spawn_rooted_log("Loading calendar month", {
                        let m = Rc::downgrade(&m);
                        let channel = channel.clone();
                        let eg = pc.eg();
                        async move {
                            'restart : loop {
                                let mut items = HashMap::new();
                                let mut token = None;
                                let mut page = 0;
                                let mut have;
                                loop {
                                    let Some(res) = req_get_retry(c2s::GetCalendarPage {
                                        channel: channel.id.clone(),
                                        month: month,
                                        page: page,
                                        token: token,
                                    }).await? else {
                                        return Ok(());
                                    };
                                    match res {
                                        CalendarPageRes::Stale => continue 'restart,
                                        CalendarPageRes::Page(p) => {
                                            token = Some(p.token);
                                            have = p.latest_activity;
                                            for msg in p.messages {
                                                items.insert(msg.original_id.message.clone(), CalItem {
                                                    channel: channel.clone(),
                                                    root: msg.original_id,
                                                    blocks: msg.message.blocks,
                                                });
                                            }
                                            if p.last {
                                                break;
                                            }
                                            page += 1;
                                        },
                                    }
                                }
                                let Some(m) = m.upgrade() else {
                                    return Ok(());
                                };
                                m.channels.borrow_mut().insert(channel.id.clone(), MonthChannel {
                                    have: have,
                                    items: items,
                                });
                                let known =
                                    state()
                                        .lookup_channel
                                        .borrow()
                                        .get(&channel.id)
                                        .and_then(|c| c.last_offset.get());
                                if let Some(known) = known {
                                    apply_activity(&eg, vec![m.clone()], channel.clone(), known).await?;
                                }
                                if let Some(cal) = m.cal.upgrade() {
                                    eg.event(|pc| render_month(pc, &cal, &m)).unwrap();
                                }
                                return Ok(());
                            }
                        }
                    }));
                }
                m
            },
        };
        let out = rooting::el("div").push(m.el.clone());
        out.ref_own(|_| m);
        return out;
    }

    fn time(&self) -> Self::Time {
        return self.index;
    }
}

struct MonthFeed {
    cal: Weak<Calendar_>,
    parent: Rc<RefCell<Option<WeakInfinite<MonthEntry>>>>,
}

const MONTH_FEED_BATCH: i64 = 4;

impl Feed<MonthEntry> for MonthFeed {
    fn set_parent(&self, parent: WeakInfinite<MonthEntry>) {
        *self.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, _eg: EventGraph, time: MonthIndex) {
        let parent = self.parent.clone();
        let cal = self.cal.clone();
        spawn_local(async move {
            let Some(parent) = parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                return;
            };
            let entries = (time.0 - MONTH_FEED_BATCH ..= time.0 + MONTH_FEED_BATCH).map(|i| Rc::new(MonthEntry {
                cal: cal.clone(),
                index: MonthIndex(i),
            })).collect();
            parent.respond_entries_around(&(), &time, entries, false, false);
        });
    }

    fn request_before(&self, _eg: EventGraph, time: MonthIndex) {
        let parent = self.parent.clone();
        let cal = self.cal.clone();
        spawn_local(async move {
            let Some(parent) = parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                return;
            };
            let entries = (1 ..= MONTH_FEED_BATCH).map(|i| Rc::new(MonthEntry {
                cal: cal.clone(),
                index: MonthIndex(time.0 - i),
            })).collect();
            parent.respond_entries_before(&(), &time, entries, false);
        });
    }

    fn request_after(&self, _eg: EventGraph, time: MonthIndex) {
        let parent = self.parent.clone();
        let cal = self.cal.clone();
        spawn_local(async move {
            let Some(parent) = parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                return;
            };
            let entries = (1 ..= MONTH_FEED_BATCH).map(|i| Rc::new(MonthEntry {
                cal: cal.clone(),
                index: MonthIndex(time.0 + i),
            })).collect();
            parent.respond_entries_after(&(), &time, entries, false);
        });
    }
}

struct DayOccurrence {
    item: CalItem,
    start: Timestamp,
}

fn occurrence_local_days(o: &Occurrence, tz: &TimeZone) -> Result<(Date, Date, Timestamp), String> {
    match o {
        Occurrence::AllDay { start, end, .. } => {
            return Ok((*start, *end, start.to_zoned(tz.clone()).map_err(|e| e.to_string())?.timestamp()));
        },
        Occurrence::Timed { start, end, .. } => {
            let start_date = start.to_zoned(tz.clone()).date();
            let end_date = if end > start {
                end.checked_sub(1.nanosecond()).unwrap().to_zoned(tz.clone()).date()
            } else {
                start_date
            };
            return Ok((start_date, end_date, *start));
        },
    }
}

fn month_items(m: &CalMonth_) -> Vec<CalItem> {
    let mut out = vec![];
    for c in m.channels.borrow().values() {
        out.extend(c.items.values().cloned());
    }
    return out;
}

fn render_month(pc: &mut ProcessingContext, cal: &Rc<Calendar_>, m: &CalMonth_) {
    let Ok(first) = month_start(m.month.year, m.month.month) else {
        return;
    };
    let next = first.checked_add(1.month()).unwrap();
    let days = first.days_in_month() as usize;
    let mut bars = vec![
        false;
        days
    ];
    let mut dots = vec![
        false;
        days
    ];
    if let Ok(window) = CalendarWindow::local(first, next, &cal.tz) {
        for item in month_items(m) {
            for block in &item.blocks {
                let MessageBlock::Event(event) = block else {
                    continue;
                };
                let occurrences = match event_occurrences(event, &window) {
                    Ok(o) => o,
                    Err(e) => {
                        state().log.log(&format!("Error expanding calendar event: {}", e));
                        continue;
                    },
                };
                for o in occurrences {
                    let (start, end, _) = match occurrence_local_days(&o, &cal.tz) {
                        Ok(v) => v,
                        Err(e) => {
                            state().log.log(&format!("Error placing calendar event: {}", e));
                            continue;
                        },
                    };
                    let spanning = start != end;
                    let mut d = start.max(first);
                    while d <= end && d < next {
                        let i = (d.day() - 1) as usize;
                        if spanning {
                            bars[i] = true;
                        } else {
                            dots[i] = true;
                        }
                        d = d.tomorrow().unwrap();
                    }
                }
            }
        }
    }
    let column = |d: Date| -> i8 {
        return (d.weekday().to_monday_zero_offset() - cal.week_start.to_monday_zero_offset()).rem_euclid(7) + 1;
    };
    let mut children = vec![];
    for i in 0 .. days {
        let d = first.checked_add((i as i32).days()).unwrap();
        let day = style_export::leaf_calendar_day(style_export::LeafCalendarDayArgs {
            day: crate::js::intl_date(d, &[("day", "numeric")]),
            column: if i == 0 {
                Some(column(d).to_string())
            } else {
                None
            },
            past: d < cal.today,
            today: d == cal.today,
            first: i == 0,
            last: i + 1 == days,
            bar: bars[i],
            dot: dots[i],
        }).root;
        day.ref_on("click", {
            let cal = Rc::downgrade(cal);
            let eg = pc.eg();
            move |_| {
                let Some(cal) = cal.upgrade() else {
                    return;
                };
                eg.event(|pc| {
                    let cal = &cal;
                    let month = MonthIndex::of(d).month();
                    let items =
                        cal
                            .months
                            .borrow()
                            .get(&month)
                            .and_then(|m| m.upgrade())
                            .map(|m| month_items(&m))
                            .unwrap_or_default();
                    let mut found = vec![];
                    if let Ok(window) = CalendarWindow::local(d, d.tomorrow().unwrap(), &cal.tz) {
                        for item in items {
                            let mut first: Option<Timestamp> = None;
                            for block in &item.blocks {
                                let MessageBlock::Event(event) = block else {
                                    continue;
                                };
                                let Ok(occurrences) = event_occurrences(event, &window) else {
                                    continue;
                                };
                                for o in occurrences {
                                    let Ok((_, _, start)) = occurrence_local_days(&o, &cal.tz) else {
                                        continue;
                                    };
                                    if first.map(|f| start < f).unwrap_or(true) {
                                        first = Some(start);
                                    }
                                }
                            }
                            if let Some(start) = first {
                                found.push(DayOccurrence {
                                    item: item,
                                    start: start,
                                });
                            }
                        }
                    }
                    found.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.item.root.cmp(&b.item.root)));
                    let selected: Rc<RefCell<Option<(CalItem, WeakEl)>>> = Rc::new(RefCell::new(None));
                    let own_identities =
                        localdata::get_stored_api_identities(None)
                            .into_iter()
                            .map(|i| i.res.id)
                            .collect::<BTreeSet<_>>();
                    let mut children = vec![];
                    for f in found {
                        let m_el =
                            style_export::cont_chat_entry_mode_message(style_export::ContChatEntryModeMessageArgs {
                                left: !own_identities.contains(&f.item.root.message.identity),
                                date: f.start.to_string(),
                            });
                        let portrait = identity_portrait(pc, &f.item.root.message.identity);
                        m_el
                            .portrait
                            .ref_own(
                                |portrait_el| link!(
                                    (_pc = pc),
                                    (portrait = portrait),
                                    (),
                                    (portrait_el = portrait_el.weak(), sender = f.item.root.message.identity.clone()) {
                                        let portrait_el = portrait_el.upgrade()?;
                                        let img = identity_image(&sender, &*portrait.borrow());
                                        portrait_el.ref_clear();
                                        portrait_el.ref_push(
                                            style_export::leaf_chat_entry_portrait(
                                                style_export::LeafChatEntryPortraitArgs {
                                                    image: img.image,
                                                    generated_image: img.generated_image,
                                                }
                                            ).root
                                        );
                                    }
                                ),
                            );
                        build_message_body(&m_el.body, &view_blocks(&f.item.root.channel, f.item.blocks.clone()));
                        m_el.root.ref_on("click", {
                            let selected = selected.clone();
                            let item = f.item.clone();
                            let el = m_el.root.weak();
                            move |_| {
                                let Some(el) = el.upgrade() else {
                                    return;
                                };
                                let selected_class = style_export::class_state_selected().value;
                                if let Some((_, old)) = selected.borrow_mut().take() {
                                    if let Some(old) = old.upgrade() {
                                        old.ref_modify_classes(&[(&selected_class, false)]);
                                    }
                                }
                                el.ref_modify_classes(&[(&selected_class, true)]);
                                *selected.borrow_mut() = Some((item.clone(), el.weak()));
                            }
                        });
                        children.push(m_el.root);
                    }
                    if children.is_empty() {
                        children.push(style_export::leaf_calendar_day_empty().root);
                    }
                    let head = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
                        back_link: ministate_octothorpe(&state().ministate.borrow()),
                        center: style_export::leaf_nonchat_head_bar_center(
                            style_export::LeafNonchatHeadBarCenterArgs {
                                text: crate::js::intl_date(
                                    d,
                                    &[
                                        ("weekday", "long"),
                                        ("day", "numeric"),
                                        ("month", "long"),
                                        ("year", "numeric"),
                                    ],
                                ),
                                link: None,
                            },
                        ).root,
                        right: None,
                    });
                    let control = match &cal.action {
                        DayAction::GoTo => {
                            let b = style_export::leaf_calendar_day_button_go_to().root;
                            b.ref_on("click", {
                                let selected = selected.clone();
                                let eg = pc.eg();
                                move |_| {
                                    let Some((item, _)) = selected.borrow().clone() else {
                                        return;
                                    };
                                    eg.event(|pc| {
                                        goto_replace_ministate(
                                            pc,
                                            &state().log,
                                            &Ministate::Channel(MinistateChannel {
                                                id: item.channel.id.clone(),
                                                own_identity: item.channel.own_identity.clone(),
                                                reset_id: Some(item.root.clone()),
                                            }),
                                        );
                                    }).unwrap();
                                }
                            });
                            b
                        },
                        DayAction::Select(_) => style_export::leaf_calendar_day_button_select().root,
                    };
                    let page = style_export::cont_calendar_page(style_export::ContCalendarPageArgs {
                        head_bar: head.root,
                        body: style_export::cont_calendar_day_list(
                            style_export::ContCalendarDayListArgs { children: children },
                        ).root,
                        controls: Some(
                            style_export::cont_chat_controls_bar_mode_menu(
                                style_export::ContChatControlsBarModeMenuArgs { children: vec![control.clone()] },
                            ).root,
                        ),
                    }).root;
                    let close: Rc<dyn Fn()> = match &cal.layer {
                        DayLayer::Page(stack) => {
                            stack.ref_push(page.clone());
                            let stack = stack.weak();
                            let page = page.weak();
                            Rc::new(move || {
                                let (Some(stack), Some(page)) = (stack.upgrade(), page.upgrade()) else {
                                    return;
                                };
                                remove_child(&stack, &page);
                            })
                        },
                        DayLayer::Modal => {
                            let modal = style_export::cont_calendar_modal().root;
                            modal.ref_push(page.clone());
                            let handle = ModalHandle::new_page(&modal);
                            handle.open();
                            Rc::new(move || handle.close())
                        },
                    };
                    on_head_back(&head.back, {
                        let close = close.clone();
                        move || close()
                    });
                    if let DayAction::Select(on_select) = &cal.action {
                        control.ref_on("click", {
                            let on_select = on_select.clone();
                            let close = close.clone();
                            move |_| {
                                close();
                                on_select(d);
                            }
                        });
                    }
                }).unwrap();
            }
        });
        children.push(day);
    }
    let last = first.checked_add(((days - 1) as i32).days()).unwrap();
    m.el.ref_replace(vec![style_export::cont_calendar_month(style_export::ContCalendarMonthArgs {
        children: children,
        overlap_next: column(last) != 7,
    }).root]);
}

async fn apply_activity(
    eg: &EventGraph,
    months: Vec<CalMonth>,
    channel: CalChannel,
    to: ActivityOffset,
) -> Result<(), String> {
    let mut at = None;
    for m in &months {
        let have = m.channels.borrow().get(&channel.id).and_then(|c| c.have);
        match (at, have) {
            (_, None) => {
                at = None;
                break;
            },
            (None, Some(h)) => at = Some(h),
            (Some(a), Some(h)) => if h.offset_pos.offset < a.offset_pos.offset {
                at = Some(h);
            },
        }
    }
    let mut at = at;
    loop {
        if let Some(a) = &at {
            if a.offset_pos.offset >= to {
                break;
            }
        }
        let next_page = match &at {
            Some(a) => match a.offset_pos.pos {
                PagePosition::Last => ActivityPage(a.page.0 + 1),
                PagePosition::First | PagePosition::Middle => a.page,
            },
            None => ActivityPage(0),
        };
        let Some(resp) = req_get_retry(c2s::GetActivityPage {
            channel: channel.id.clone(),
            page: next_page,
        }).await? else {
            break;
        };
        let mut advanced = None;
        for entry in resp.messages {
            if let Some(a) = &at {
                if entry.offset_pos.offset <= a.offset_pos.offset {
                    continue;
                }
            }
            advanced = Some(ActivityPageOffsetPos {
                page: next_page,
                offset_pos: entry.offset_pos,
            });
            let body = match serde_json::from_str::<MessageBody>(&entry.message.0.message) {
                Ok(b) => b,
                Err(e) => {
                    state().log.log(&format!("Error deserializing message from activity, skipping: {}", e));
                    continue;
                },
            };
            for m in &months {
                let mut channels = m.channels.borrow_mut();
                let Some(mc) = channels.get_mut(&channel.id) else {
                    continue;
                };
                if mc.have.map(|h| entry.offset_pos.offset <= h.offset_pos.offset).unwrap_or(false) {
                    continue;
                }
                let root = match &body.rel {
                    MessageRel::EditOf(target) => target.clone(),
                    MessageRel::None | MessageRel::ReplyTo(_) => body.id.clone(),
                };
                let has_event = body.blocks.iter().any(|b| matches!(b, MessageBlock::Event(_)));
                match mc.items.get_mut(&root) {
                    Some(existing) => {
                        existing.blocks = body.blocks.clone();
                    },
                    None => {
                        if has_event {
                            mc.items.insert(root.clone(), CalItem {
                                channel: channel.clone(),
                                root: QualifiedMessageId {
                                    channel: channel.id.clone(),
                                    message: root,
                                },
                                blocks: body.blocks.clone(),
                            });
                        }
                    },
                }
                mc.have = advanced;
            }
        }
        let Some(advanced) = advanced else {
            break;
        };
        at = Some(advanced);
    }
    for m in &months {
        if let Some(cal) = m.cal.upgrade() {
            eg.event(|pc| render_month(pc, &cal, m)).unwrap();
        }
    }
    return Ok(());
}

impl Calendar {
    pub fn notify(&self, channel: &QualifiedChannelId, offset: ActivityOffset) {
        let Some(cal_channel) = self.0.channels.iter().find(|c| c.id == *channel).cloned() else {
            return;
        };
        let months = self.0.months.borrow().values().filter_map(|m| m.upgrade()).filter(|m| {
            return m.channels.borrow().contains_key(channel);
        }).collect::<Vec<_>>();
        if months.is_empty() {
            return;
        }
        self.0.activity.borrow_mut().insert(channel.clone(), spawn_rooted_log("Applying new activity to calendar", {
            let eg = self.0.eg.clone();
            async move {
                return apply_activity(&eg, months, cal_channel, offset).await;
            }
        }));
    }
}

pub fn build_calendar(
    pc: &mut ProcessingContext,
    layer: DayLayer,
    channels: Vec<CalChannel>,
    action: DayAction,
    start: Date,
) -> (Calendar, Infinite<MonthEntry>) {
    let tz = local_tz();
    let cal = Rc::new(Calendar_ {
        eg: pc.eg(),
        channels: channels,
        today: Zoned::now().with_time_zone(tz.clone()).date(),
        tz: tz,
        week_start: week_start(),
        layer: layer,
        action: action,
        months: RefCell::new(HashMap::new()),
        activity: RefCell::new(HashMap::new()),
    });
    let inf = Infinite::new(&pc.eg(), InfiniteEls {
        center_spinner: style_export::leaf_chat_spinner_center().root,
        early_spinner: style_export::leaf_chat_spinner_early().root,
        late_spinner: style_export::leaf_chat_spinner_late().root,
    });
    inf.el().ref_classes(&[&style_export::class_page_chat().value]);
    inf.add_feed((), MonthFeed {
        cal: Rc::downgrade(&cal),
        parent: Rc::new(RefCell::new(None)),
    });
    inf.jump_to(&MonthIndex::of(start));
    return (Calendar(cal), inf);
}

pub fn group_channels(group: &shared::interface::shared::ChannelGroupId) -> Vec<CalChannel> {
    return get_stored_api_channels(None)
        .into_iter()
        .filter(|c| c.res.group.as_ref() == Some(group))
        .map(|c| CalChannel {
            id: c.res.id,
            own_identity: c.res.own_identity,
        })
        .collect();
}

pub fn on_head_back(back: &El, f: impl 'static + Fn()) {
    back.ref_on("click", move |ev| {
        ev.prevent_default();
        f();
    });
}

pub fn open_date_picker(
    pc: &mut ProcessingContext,
    channel: CalChannel,
    initial: Date,
    on_select: Rc<dyn Fn(Date)>,
) {
    let modal = style_export::cont_calendar_modal().root;
    let handle = ModalHandle::new_page(&modal);
    let select = Rc::new({
        let handle = handle.clone();
        let on_select = on_select.clone();
        move |d: Date| {
            handle.close();
            on_select(d);
        }
    });
    let head_center = style_export::leaf_nonchat_head_bar_center_placeholder().root;
    let name_load = get_or_req_channel(&pc.eg(), &channel.id, false).then(pc, {
        let head_center = head_center.weak();
        move |_pc, res| {
            let Some(head_center) = head_center.upgrade() else {
                return;
            };
            match res {
                Ok(Some(c)) => {
                    head_center.ref_text(&c.name.get());
                },
                Ok(None) => {
                    head_center.ref_text("");
                },
                Err(e) => {
                    state().log.log(&format!("Error looking up calendar channel: {}", e));
                },
            }
        }
    });
    head_center.ref_own(|_| name_load);
    let select_calendar = style_export::leaf_calendar_head_bar_button_select_calendar().root;
    let head = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&state().ministate.borrow()),
        center: head_center,
        right: Some(select_calendar.clone()),
    });
    let (cal, inf) = build_calendar(pc, DayLayer::Modal, vec![channel], DayAction::Select(select), initial);
    on_head_back(&head.back, {
        let handle = handle.clone();
        move || handle.close()
    });
    select_calendar.ref_on("click", {
        let handle = handle.clone();
        let eg = pc.eg();
        let on_select = on_select.clone();
        move |_| {
            let list_modal = style_export::cont_calendar_modal().root;
            let list_handle = ModalHandle::new_page(&list_modal);
            let mut buttons = vec![];
            for c in get_stored_api_channels(None) {
                let b =
                    style_export::leaf_menu_button(
                        style_export::LeafMenuButtonArgs { text: c.res.name.clone() },
                    ).root;
                b.ref_on("click", {
                    let handle = handle.clone();
                    let list_handle = list_handle.clone();
                    let eg = eg.clone();
                    let on_select = on_select.clone();
                    let channel = CalChannel {
                        id: c.res.id.clone(),
                        own_identity: c.res.own_identity.clone(),
                    };
                    move |_| {
                        list_handle.close();
                        handle.close();
                        eg.event(|pc| open_date_picker(pc, channel.clone(), initial, on_select.clone())).unwrap();
                    }
                });
                buttons.push(b);
            }
            let list_head = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
                back_link: ministate_octothorpe(&state().ministate.borrow()),
                center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                    text: format!("Select calendar"),
                    link: None,
                }).root,
                right: None,
            });
            on_head_back(&list_head.back, {
                let list_handle = list_handle.clone();
                move || list_handle.close()
            });
            list_modal.ref_push(style_export::cont_page_menu(style_export::ContPageMenuArgs {
                head_bar: list_head.root,
                children: buttons,
            }).root);
            list_handle.open();
        }
    });
    let page = style_export::cont_calendar_page(style_export::ContCalendarPageArgs {
        head_bar: head.root,
        body: inf.el(),
        controls: None,
    }).root;
    page.ref_own(|_| (cal, inf));
    modal.ref_push(page);
    handle.open();
}
