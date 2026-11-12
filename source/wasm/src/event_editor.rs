use {
    crate::{
        calendar::{
            CalChannel,
            format_date,
            local_tz,
            on_head_back,
            open_date_picker,
            week_start,
        },
        js::{
            intl_timestamp,
            intl_weekday,
            style_export,
        },
        modal::ModalHandle,
        pageutil::build_form_inner,
        state::{
            ministate_octothorpe,
            state,
        },
    },
    jiff::{
        ToSpan,
        Zoned,
        civil::{
            Date,
            DateTime,
            Time,
        },
        tz::TimeZone,
    },
    rooting::{
        El,
        el,
    },
    rooting_forms::{
        Form,
        FormElements,
        FormState,
        FormWith,
        css::{
            ATTR_LABEL,
            CSS_CLASS_SMALL_INPUT,
            err_el,
        },
    },
    shared::{
        calendar::{
            validate_event,
            weekday_from_jiff,
            weekday_to_jiff,
        },
        interface::shared::{
            EventMonthly,
            EventRange,
            EventRangeAllDay,
            EventRangeTimed,
            EventRepeat,
            EventRepeatEnd,
            EventRepeatRule,
            EventWeekday,
            MessageBlockEvent,
        },
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::BTreeSet,
        rc::Rc,
    },
    wasm_bindgen::JsCast,
    web_sys::HtmlInputElement,
};

fn weekday_short(w: EventWeekday) -> String {
    return intl_weekday(weekday_to_jiff(w), "short");
}

fn time_text(t: jiff::Timestamp) -> String {
    return intl_timestamp(t, &[("hour", "numeric"), ("minute", "2-digit")]);
}

pub fn event_timeline_text(event: &MessageBlockEvent) -> Result<String, String> {
    let mut parts = vec![];
    let start_date;
    match &event.range {
        EventRange::AllDay(r) => {
            start_date = r.start;
            if r.start == r.end {
                parts.push(format_date(r.start));
            } else {
                parts.push(format!("{} - {}", format_date(r.start), format_date(r.end)));
            }
        },
        EventRange::Timed(r) => {
            start_date = r.start.date();
            let tz = TimeZone::get(&r.tz).map_err(|e| e.to_string())?;
            let local = local_tz();
            let s = r.start.to_zoned(tz.clone()).map_err(|e| e.to_string())?.with_time_zone(local.clone());
            let e = r.end.to_zoned(tz).map_err(|e| e.to_string())?.with_time_zone(local);
            if s.date() == e.date() {
                parts.push(
                    format!("{} {} - {}", format_date(s.date()), time_text(s.timestamp()), time_text(e.timestamp())),
                );
            } else {
                parts.push(
                    format!(
                        "{} {} - {} {}",
                        format_date(s.date()),
                        time_text(s.timestamp()),
                        format_date(e.date()),
                        time_text(e.timestamp())
                    ),
                );
            }
        },
    }
    if let Some(repeat) = &event.repeat {
        let unit = match &repeat.rule {
            EventRepeatRule::Days => "day",
            EventRepeatRule::Weeks(_) => "week",
            EventRepeatRule::Months(_) => "month",
            EventRepeatRule::Years => "year",
        };
        let mut every = if repeat.interval == 1 {
            format!("every {}", unit)
        } else {
            format!("every {} {}s", repeat.interval, unit)
        };
        match &repeat.rule {
            EventRepeatRule::Days | EventRepeatRule::Years => { },
            EventRepeatRule::Weeks(days) => {
                let days = days.iter().copied().collect::<BTreeSet<_>>();
                every.push_str(
                    &format!(" on {}", days.into_iter().map(weekday_short).collect::<Vec<_>>().join(", ")),
                );
            },
            EventRepeatRule::Months(m) => match m {
                EventMonthly::DayOfMonth => every.push_str(&format!(" on day {}", start_date.day())),
                EventMonthly::NthWeekday => every.push_str(&format!(" on the {} {}", {
                    let n = (start_date.day() - 1) / 7 + 1;
                    format!("{}{}", n, match n {
                        1 => "st",
                        2 => "nd",
                        3 => "rd",
                        _ => "th",
                    })
                }, weekday_short(weekday_from_jiff(start_date.weekday())))),
                EventMonthly::LastWeekday => every.push_str(
                    &format!(" on the last {}", weekday_short(weekday_from_jiff(start_date.weekday()))),
                ),
            },
        }
        parts.push(every);
        match &repeat.end {
            EventRepeatEnd::Never => { },
            EventRepeatEnd::Until(d) => parts.push(format!("until {}", format_date(*d))),
            EventRepeatEnd::Count(c) => parts.push(if *c == 1 {
                format!("once")
            } else {
                format!("{} times", c)
            }),
        }
    }
    return Ok(parts.join(", "));
}

pub fn default_event() -> Result<MessageBlockEvent, String> {
    let tz = local_tz();
    let tz_name = tz.iana_name().ok_or_else(|| format!("Browser time zone has no IANA name"))?.to_string();
    let now = Zoned::now().with_time_zone(tz).datetime();
    let start =
        now
            .date()
            .to_datetime(Time::new(now.hour(), 0, 0, 0).unwrap())
            .checked_add(1.hour())
            .map_err(|e| e.to_string())?;
    return Ok(MessageBlockEvent {
        range: EventRange::Timed(EventRangeTimed {
            start: start,
            end: start.checked_add(1.hour()).map_err(|e| e.to_string())?,
            tz: tz_name,
        }),
        repeat: None,
    });
}

thread_local!{
    static PICKER_CHANNEL: RefCell<Option<CalChannel>> = RefCell::new(None);
}

#[derive(Clone)]
struct FormPickDate(Date);

struct FormPickDateState(Rc<Cell<Date>>);

impl FormState<FormPickDate> for FormPickDateState {
    fn parse(&self) -> Result<FormPickDate, ()> {
        return Ok(FormPickDate(self.0.get()));
    }
}

impl<C> FormWith<C> for FormPickDate {
    fn new_form_with_(
        _context: &C,
        field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (FormElements, Box<dyn FormState<Self>>) {
        let value = Rc::new(Cell::new(match from {
            Some(v) => v.0,
            None => Zoned::now().with_time_zone(local_tz()).date(),
        }));
        let input =
            el("input")
                .classes(&[CSS_CLASS_SMALL_INPUT])
                .attr(ATTR_LABEL, field)
                .attr("type", "text")
                .attr("readonly", "")
                .attr("value", &format_date(value.get()));
        input.ref_on("click", {
            let value = value.clone();
            let input = input.weak();
            move |_| {
                let Some(channel) = PICKER_CHANNEL.with(|c| c.borrow().clone()) else {
                    return;
                };
                let on_select = Rc::new({
                    let value = value.clone();
                    let input = input.clone();
                    move |d: Date| {
                        value.set(d);
                        if let Some(input) = input.upgrade() {
                            input.raw().dyn_into::<HtmlInputElement>().unwrap().set_value(&format_date(d));
                        }
                    }
                });
                state().eg.event(|pc| open_date_picker(pc, channel, value.get(), on_select)).unwrap();
            }
        });
        return (FormElements {
            error: None,
            elements: vec![input],
        }, Box::new(FormPickDateState(value)));
    }
}

#[derive(Clone)]
struct FormTime(Time);

struct FormTimeState {
    input: El,
    err_el: El,
}

impl FormState<FormTime> for FormTimeState {
    fn parse(&self) -> Result<FormTime, ()> {
        match Time::strptime("%H:%M", &self.input.raw().dyn_into::<HtmlInputElement>().unwrap().value()) {
            Ok(t) => {
                self.err_el.ref_text("");
                return Ok(FormTime(t));
            },
            Err(e) => {
                self.err_el.ref_text(&e.to_string());
                return Err(());
            },
        }
    }
}

impl<C> FormWith<C> for FormTime {
    fn new_form_with_(
        _context: &C,
        field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (FormElements, Box<dyn FormState<Self>>) {
        let err_el = err_el();
        let input =
            el("input")
                .classes(&[CSS_CLASS_SMALL_INPUT])
                .attr(ATTR_LABEL, field)
                .attr("type", "time")
                .attr("value", &match from {
                    Some(v) => v.0,
                    None => Zoned::now().with_time_zone(local_tz()).time(),
                }.strftime("%H:%M").to_string());
        return (FormElements {
            error: Some(err_el.clone()),
            elements: vec![input.clone()],
        }, Box::new(FormTimeState {
            input: input,
            err_el: err_el,
        }));
    }
}

#[derive(Clone)]
struct FormWeekdays(BTreeSet<EventWeekday>);

struct FormWeekdaysState(Vec<(EventWeekday, El)>);

impl FormState<FormWeekdays> for FormWeekdaysState {
    fn parse(&self) -> Result<FormWeekdays, ()> {
        let mut out = BTreeSet::new();
        for (day, input) in &self.0 {
            if input.raw().dyn_into::<HtmlInputElement>().unwrap().checked() {
                out.insert(*day);
            }
        }
        return Ok(FormWeekdays(out));
    }
}

impl<C> FormWith<C> for FormWeekdays {
    fn new_form_with_(
        _context: &C,
        _field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (FormElements, Box<dyn FormState<Self>>) {
        let first = week_start().to_monday_zero_offset();
        let mut inputs = vec![];
        let row = el("div").classes(&[CSS_CLASS_SMALL_INPUT]);
        for i in 0 .. 7 {
            let day = jiff::civil::Weekday::from_monday_zero_offset((first + i).rem_euclid(7)).unwrap();
            let long = intl_weekday(day, "long");
            let input = el("input").attr("type", "checkbox").attr(ATTR_LABEL, &long);
            if from.map(|f| f.0.contains(&weekday_from_jiff(day))).unwrap_or(false) {
                input.ref_attr("checked", "checked");
            }
            row.ref_push(
                el("label")
                    .attr("title", &long)
                    .push(input.clone())
                    .push(el("span").text(&intl_weekday(day, "narrow"))),
            );
            inputs.push((weekday_from_jiff(day), input));
        }
        return (FormElements {
            error: None,
            elements: vec![row],
        }, Box::new(FormWeekdaysState(inputs)));
    }
}

#[derive(rooting_forms::Form, Clone)]
struct FormRangeTimed {
    #[title("Start date")]
    start_date: FormPickDate,
    #[title("Start time")]
    start_time: FormTime,
    #[title("End date")]
    end_date: FormPickDate,
    #[title("End time")]
    end_time: FormTime,
}

#[derive(rooting_forms::Form, Clone)]
struct FormRangeAllDay {
    #[title("Start")]
    start: FormPickDate,
    #[title("End")]
    end: FormPickDate,
}

#[derive(rooting_forms::Form, Clone)]
enum FormRange {
    #[title("At a time")]
    Timed(FormRangeTimed),
    #[title("All day")]
    AllDay(FormRangeAllDay),
}

#[derive(rooting_forms::Form, Clone)]
enum FormMonthly {
    #[title("Same day of the month")]
    DayOfMonth,
    #[title("Same weekday and week of the month")]
    NthWeekday,
    #[title("Last of that weekday in the month")]
    LastWeekday,
}

#[derive(rooting_forms::Form, Clone)]
enum FormRepeatEnd {
    #[title("Never")]
    Never,
    #[title("On date")]
    Until(FormPickDate),
    #[title("After a number of times")]
    Count(u32),
}

#[derive(rooting_forms::Form, Clone)]
struct FormRepeatSimple {
    #[title("Every")]
    interval: u32,
    #[title("Ends")]
    end: FormRepeatEnd,
}

#[derive(rooting_forms::Form, Clone)]
struct FormRepeatWeeks {
    #[title("Every")]
    interval: u32,
    #[title("On")]
    days: FormWeekdays,
    #[title("Ends")]
    end: FormRepeatEnd,
}

#[derive(rooting_forms::Form, Clone)]
struct FormRepeatMonths {
    #[title("Every")]
    interval: u32,
    #[title("On")]
    by: FormMonthly,
    #[title("Ends")]
    end: FormRepeatEnd,
}

#[derive(rooting_forms::Form, Clone)]
enum FormRepeat {
    #[title("Doesn't repeat")]
    None,
    #[title("Daily")]
    Days(FormRepeatSimple),
    #[title("Weekly")]
    Weeks(FormRepeatWeeks),
    #[title("Monthly")]
    Months(FormRepeatMonths),
    #[title("Yearly")]
    Years(FormRepeatSimple),
}

#[derive(rooting_forms::Form, Clone)]
struct EventForm_ {
    #[title("When")]
    range: FormRange,
    #[title("Repeat")]
    repeat: FormRepeat,
}

fn to_local(dt: DateTime, from: &TimeZone, local: &TimeZone) -> Result<DateTime, String> {
    return Ok(dt.to_zoned(from.clone()).map_err(|e| e.to_string())?.with_time_zone(local.clone()).datetime());
}

pub fn open_event_editor(channel: CalChannel, initial: &MessageBlockEvent, on_done: Rc<dyn Fn(MessageBlockEvent)>) {
    let local = local_tz();
    let Some(tz) = local.iana_name().map(|n| n.to_string()) else {
        state().log.log("Browser time zone has no IANA name, can't edit events");
        return;
    };
    let range = match &initial.range {
        EventRange::Timed(r) => {
            let converted = TimeZone::get(&r.tz).map_err(|e| e.to_string()).and_then(|event_tz| {
                return Ok((to_local(r.start, &event_tz, &local)?, to_local(r.end, &event_tz, &local)?));
            });
            match converted {
                Ok((start, end)) => FormRange::Timed(FormRangeTimed {
                    start_date: FormPickDate(start.date()),
                    start_time: FormTime(start.time()),
                    end_date: FormPickDate(end.date()),
                    end_time: FormTime(end.time()),
                }),
                Err(e) => {
                    state().log.log(&format!("Can't convert event time zone [{}] to local: {}", r.tz, e));
                    return;
                },
            }
        },
        EventRange::AllDay(r) => FormRange::AllDay(FormRangeAllDay {
            start: FormPickDate(r.start),
            end: FormPickDate(r.end),
        }),
    };
    PICKER_CHANNEL.with(|c| *c.borrow_mut() = Some(channel));
    let from = EventForm_ {
        range: range,
        repeat: match &initial.repeat {
            None => FormRepeat::None,
            Some(repeat) => {
                let end = match &repeat.end {
                    EventRepeatEnd::Never => FormRepeatEnd::Never,
                    EventRepeatEnd::Until(d) => FormRepeatEnd::Until(FormPickDate(*d)),
                    EventRepeatEnd::Count(c) => FormRepeatEnd::Count(*c),
                };
                match &repeat.rule {
                    EventRepeatRule::Days => FormRepeat::Days(FormRepeatSimple {
                        interval: repeat.interval,
                        end: end,
                    }),
                    EventRepeatRule::Weeks(days) => FormRepeat::Weeks(FormRepeatWeeks {
                        interval: repeat.interval,
                        days: FormWeekdays(days.iter().copied().collect()),
                        end: end,
                    }),
                    EventRepeatRule::Months(m) => FormRepeat::Months(FormRepeatMonths {
                        interval: repeat.interval,
                        by: match m {
                            EventMonthly::DayOfMonth => FormMonthly::DayOfMonth,
                            EventMonthly::NthWeekday => FormMonthly::NthWeekday,
                            EventMonthly::LastWeekday => FormMonthly::LastWeekday,
                        },
                        end: end,
                    }),
                    EventRepeatRule::Years => FormRepeat::Years(FormRepeatSimple {
                        interval: repeat.interval,
                        end: end,
                    }),
                }
            },
        },
    };
    let (form_els, form_state) = EventForm_::new_form("", Some(&from));
    let head_bar = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&state().ministate.borrow()),
        center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
            text: format!("Event"),
            link: None,
        }).root,
        right: None,
    });
    let form = style_export::cont_page_form(style_export::ContPageFormArgs {
        head_bar: head_bar.root,
        children: vec![],
    });
    let modal = style_export::cont_calendar_modal().root;
    let handle = ModalHandle::new_page(&modal);
    on_head_back(&head_bar.back, {
        let handle = handle.clone();
        move || handle.close()
    });
    form.errors.ref_extend(form_els.error.into_iter().collect());
    form.body.ref_push(build_form_inner(&form.submit, form.errors, form_els.elements, {
        let handle = handle.clone();
        let form_state = Rc::new(form_state);
        async move |_idem| {
            let Ok(v) = form_state.parse() else {
                return Ok(());
            };
            let end_of = |end: FormRepeatEnd| match end {
                FormRepeatEnd::Never => EventRepeatEnd::Never,
                FormRepeatEnd::Until(d) => EventRepeatEnd::Until(d.0),
                FormRepeatEnd::Count(c) => EventRepeatEnd::Count(c),
            };
            let range = match v.range {
                FormRange::Timed(r) => EventRange::Timed(EventRangeTimed {
                    start: DateTime::from_parts(r.start_date.0, r.start_time.0),
                    end: DateTime::from_parts(r.end_date.0, r.end_time.0),
                    tz: tz.clone(),
                }),
                FormRange::AllDay(r) => EventRange::AllDay(EventRangeAllDay {
                    start: r.start.0,
                    end: r.end.0,
                }),
            };
            let start_weekday = match &range {
                EventRange::Timed(r) => r.start.weekday(),
                EventRange::AllDay(r) => r.start.weekday(),
            };
            let event = MessageBlockEvent {
                range: range,
                repeat: match v.repeat {
                    FormRepeat::None => None,
                    FormRepeat::Days(r) => Some(EventRepeat {
                        interval: r.interval,
                        rule: EventRepeatRule::Days,
                        end: end_of(r.end),
                    }),
                    FormRepeat::Weeks(r) => {
                        let mut days = r.days.0;
                        days.insert(weekday_from_jiff(start_weekday));
                        Some(EventRepeat {
                            interval: r.interval,
                            rule: EventRepeatRule::Weeks(days.into_iter().collect()),
                            end: end_of(r.end),
                        })
                    },
                    FormRepeat::Months(r) => Some(EventRepeat {
                        interval: r.interval,
                        rule: EventRepeatRule::Months(match r.by {
                            FormMonthly::DayOfMonth => EventMonthly::DayOfMonth,
                            FormMonthly::NthWeekday => EventMonthly::NthWeekday,
                            FormMonthly::LastWeekday => EventMonthly::LastWeekday,
                        }),
                        end: end_of(r.end),
                    }),
                    FormRepeat::Years(r) => Some(EventRepeat {
                        interval: r.interval,
                        rule: EventRepeatRule::Years,
                        end: end_of(r.end),
                    }),
                },
            };
            validate_event(&event)?;
            handle.close();
            on_done(event);
            return Ok(());
        }
    }));
    modal.ref_push(form.root);
    handle.open();
}
