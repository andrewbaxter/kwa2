use {
    crate::interface::shared::{
        EventMonthly,
        EventRange,
        EventRepeat,
        EventRepeatEnd,
        EventRepeatRule,
        EventWeekday,
        MessageBlockEvent,
    },
    jiff::{
        civil::{
            Date,
            DateTime,
            Weekday,
        },
        tz::TimeZone,
        SignedDuration,
        Span,
        Timestamp,
        ToSpan,
    },
};

pub const TZ_MARGIN: SignedDuration = SignedDuration::from_hours(14);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CalendarWindow {
    pub start: Timestamp,
    pub end: Timestamp,
    pub start_date: Date,
    pub end_date: Date,
}

impl CalendarWindow {
    pub fn utc_widened(start_date: Date, end_date: Date) -> Result<CalendarWindow, String> {
        return Ok(CalendarWindow {
            start: utc_midnight(start_date)?.checked_sub(TZ_MARGIN).map_err(|e| e.to_string())?,
            end: utc_midnight(end_date)?.checked_add(TZ_MARGIN).map_err(|e| e.to_string())?,
            start_date: start_date,
            end_date: end_date,
        });
    }

    pub fn local(start_date: Date, end_date: Date, tz: &TimeZone) -> Result<CalendarWindow, String> {
        return Ok(CalendarWindow {
            start: start_date.to_zoned(tz.clone()).map_err(|e| e.to_string())?.timestamp(),
            end: end_date.to_zoned(tz.clone()).map_err(|e| e.to_string())?.timestamp(),
            start_date: start_date,
            end_date: end_date,
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Occurrence {
    AllDay {
        index: u64,
        start: Date,
        end: Date,
    },
    Timed {
        index: u64,
        start: Timestamp,
        end: Timestamp,
    },
}

impl Occurrence {
    pub fn index(&self) -> u64 {
        match self {
            Occurrence::AllDay { index, .. } => return *index,
            Occurrence::Timed { index, .. } => return *index,
        }
    }

    pub fn overlaps(&self, window: &CalendarWindow) -> bool {
        match self {
            Occurrence::AllDay { start, end, .. } => return *start < window.end_date && *end >= window.start_date,
            Occurrence::Timed { start, end, .. } => return *start < window.end &&
                (*end > window.start || *start >= window.start),
        }
    }
}

pub fn utc_midnight(d: Date) -> Result<Timestamp, String> {
    return Ok(d.to_zoned(TimeZone::UTC).map_err(|e| e.to_string())?.timestamp());
}

pub fn month_start(year: i16, month: i8) -> Result<Date, String> {
    return Date::new(year, month, 1).map_err(|e| e.to_string());
}

pub fn month_window_utc(year: i16, month: i8) -> Result<CalendarWindow, String> {
    let start = month_start(year, month)?;
    let end = start.checked_add(1.month()).map_err(|e| e.to_string())?;
    return CalendarWindow::utc_widened(start, end);
}

pub fn weekday_to_jiff(w: EventWeekday) -> Weekday {
    match w {
        EventWeekday::Monday => return Weekday::Monday,
        EventWeekday::Tuesday => return Weekday::Tuesday,
        EventWeekday::Wednesday => return Weekday::Wednesday,
        EventWeekday::Thursday => return Weekday::Thursday,
        EventWeekday::Friday => return Weekday::Friday,
        EventWeekday::Saturday => return Weekday::Saturday,
        EventWeekday::Sunday => return Weekday::Sunday,
    }
}

pub fn weekday_from_jiff(w: Weekday) -> EventWeekday {
    match w {
        Weekday::Monday => return EventWeekday::Monday,
        Weekday::Tuesday => return EventWeekday::Tuesday,
        Weekday::Wednesday => return EventWeekday::Wednesday,
        Weekday::Thursday => return EventWeekday::Thursday,
        Weekday::Friday => return EventWeekday::Friday,
        Weekday::Saturday => return EventWeekday::Saturday,
        Weekday::Sunday => return EventWeekday::Sunday,
    }
}

fn days_between(a: Date, b: Date) -> i64 {
    return b.duration_since(a).as_secs().div_euclid(86400);
}

fn add_days(d: Date, days: i64) -> Result<Date, String> {
    return d.checked_add(Span::new().try_days(days).map_err(|e| e.to_string())?).map_err(|e| e.to_string());
}

fn month_index(d: Date) -> i64 {
    return d.year() as i64 * 12 + d.month() as i64 - 1;
}

fn nth_of_month(d: Date) -> i8 {
    return (d.day() - 1) / 7 + 1;
}

struct Shape {
    start_date: Date,
    length_days: i64,
    timed: Option<(jiff::civil::Time, SignedDuration, TimeZone)>,
}

fn shape(range: &EventRange) -> Result<Shape, String> {
    match range {
        EventRange::AllDay(r) => {
            return Ok(Shape {
                start_date: r.start,
                length_days: days_between(r.start, r.end),
                timed: None,
            });
        },
        EventRange::Timed(r) => {
            let tz = TimeZone::get(&r.tz).map_err(|e| e.to_string())?;
            let duration = r.end.duration_since(r.start);
            return Ok(Shape {
                start_date: r.start.date(),
                length_days: duration.as_secs().div_euclid(86400) + 1,
                timed: Some((r.start.time(), duration, tz)),
            });
        },
    }
}

pub fn validate_event(event: &MessageBlockEvent) -> Result<(), String> {
    match &event.range {
        EventRange::AllDay(r) => {
            if r.end < r.start {
                return Err(format!("Event ends before it starts"));
            }
        },
        EventRange::Timed(r) => {
            if r.end < r.start {
                return Err(format!("Event ends before it starts"));
            }
            TimeZone::get(&r.tz).map_err(|e| format!("Invalid event time zone [{}]: {}", r.tz, e))?;
        },
    }
    let Some(repeat) = &event.repeat else {
        return Ok(());
    };
    if repeat.interval == 0 {
        return Err(format!("Event repeat interval must be at least 1"));
    }
    let start_date = shape(&event.range)?.start_date;
    match &repeat.rule {
        EventRepeatRule::Days | EventRepeatRule::Years => { },
        EventRepeatRule::Weeks(days) => {
            if days.is_empty() {
                return Err(format!("Weekly event repeat needs at least one weekday"));
            }
        },
        EventRepeatRule::Months(m) => match m {
            EventMonthly::DayOfMonth => { },
            EventMonthly::NthWeekday => {
                if nth_of_month(start_date) > 4 {
                    return Err(format!("Event start isn't within the first four weeks of the month"));
                }
            },
            EventMonthly::LastWeekday => {
                if start_date.day() + 7 <= start_date.days_in_month() {
                    return Err(format!("Event start isn't the last such weekday of the month"));
                }
            },
        },
    }
    match &repeat.end {
        EventRepeatEnd::Never => { },
        EventRepeatEnd::Until(d) => {
            if *d < start_date {
                return Err(format!("Event repeat ends before the event starts"));
            }
        },
        EventRepeatEnd::Count(c) => {
            if *c == 0 {
                return Err(format!("Event repeat count must be at least 1"));
            }
        },
    }
    return Ok(());
}

fn sorted_weekday_offsets(days: &[EventWeekday]) -> Vec<i64> {
    let mut out = days.iter().map(|d| weekday_to_jiff(*d).to_monday_zero_offset() as i64).collect::<Vec<_>>();
    out.sort();
    out.dedup();
    return out;
}

fn period_dates(start_date: Date, repeat: &EventRepeat, p: i64) -> Result<Vec<(u64, Date)>, String> {
    let interval = repeat.interval as i64;
    match &repeat.rule {
        EventRepeatRule::Days => {
            return Ok(vec![(p as u64, add_days(start_date, p * interval)?)]);
        },
        EventRepeatRule::Weeks(days) => {
            let offsets = sorted_weekday_offsets(days);
            let start_offset = start_date.weekday().to_monday_zero_offset() as i64;
            let week0 = add_days(start_date, -start_offset)?;
            let monday = add_days(week0, p * interval * 7)?;
            let first_count = offsets.iter().filter(|o| **o >= start_offset).count() as i64;
            let mut out = vec![];
            let mut j = 0i64;
            for o in offsets.iter() {
                if p == 0 && *o < start_offset {
                    continue;
                }
                let index = if p == 0 {
                    j
                } else {
                    first_count + (p - 1) * offsets.len() as i64 + j
                };
                out.push((index as u64, add_days(monday, *o)?));
                j += 1;
            }
            return Ok(out);
        },
        EventRepeatRule::Months(m) => {
            let mi = month_index(start_date) + p * interval;
            let month =
                month_start(
                    i16::try_from(mi.div_euclid(12)).map_err(|e| e.to_string())?,
                    (mi.rem_euclid(12) + 1) as i8,
                )?;
            let date = match m {
                EventMonthly::DayOfMonth => Date::new(
                    month.year(),
                    month.month(),
                    start_date.day().min(month.days_in_month()),
                ).map_err(|e| e.to_string())?,
                EventMonthly::NthWeekday => month
                    .nth_weekday_of_month(nth_of_month(start_date), start_date.weekday())
                    .map_err(|e| e.to_string())?,
                EventMonthly::LastWeekday => month
                    .nth_weekday_of_month(-1, start_date.weekday())
                    .map_err(|e| e.to_string())?,
            };
            return Ok(vec![(p as u64, date)]);
        },
        EventRepeatRule::Years => {
            let years = i16::try_from(p * interval).map_err(|e| e.to_string())?;
            return Ok(vec![(p as u64, start_date.checked_add(years.years()).map_err(|e| e.to_string())?)]);
        },
    }
}

fn period_containing(start_date: Date, repeat: &EventRepeat, d: Date) -> i64 {
    let interval = repeat.interval as i64;
    let raw = match &repeat.rule {
        EventRepeatRule::Days => days_between(start_date, d).div_euclid(interval),
        EventRepeatRule::Weeks(_) => {
            let week0 = days_between(Date::new(1970, 1, 5).unwrap(), start_date).div_euclid(7);
            let weekd = days_between(Date::new(1970, 1, 5).unwrap(), d).div_euclid(7);
            (weekd - week0).div_euclid(interval)
        },
        EventRepeatRule::Months(_) => (month_index(d) - month_index(start_date)).div_euclid(interval),
        EventRepeatRule::Years => (d.year() as i64 - start_date.year() as i64).div_euclid(interval),
    };
    return raw.max(0);
}

fn make_occurrence(shape: &Shape, index: u64, date: Date) -> Result<Occurrence, String> {
    match &shape.timed {
        None => {
            return Ok(Occurrence::AllDay {
                index: index,
                start: date,
                end: add_days(date, shape.length_days)?,
            });
        },
        Some((time, duration, tz)) => {
            let start_dt = DateTime::from_parts(date, *time);
            let end_dt = start_dt.checked_add(*duration).map_err(|e| e.to_string())?;
            return Ok(Occurrence::Timed {
                index: index,
                start: start_dt.to_zoned(tz.clone()).map_err(|e| e.to_string())?.timestamp(),
                end: end_dt.to_zoned(tz.clone()).map_err(|e| e.to_string())?.timestamp(),
            });
        },
    }
}

pub fn event_occurrences(event: &MessageBlockEvent, window: &CalendarWindow) -> Result<Vec<Occurrence>, String> {
    let shape = shape(&event.range)?;
    let (frame_start, frame_end) = match &shape.timed {
        None => (window.start_date, window.end_date),
        Some((_, _, tz)) => (
            window.start.to_zoned(tz.clone()).date(),
            add_days(window.end.to_zoned(tz.clone()).date(), 1)?,
        ),
    };
    let Some(repeat) = &event.repeat else {
        let o = make_occurrence(&shape, 0, shape.start_date)?;
        if o.overlaps(window) {
            return Ok(vec![o]);
        }
        return Ok(vec![]);
    };
    let search_from = add_days(frame_start, -(shape.length_days + 1))?;
    let mut p = (period_containing(shape.start_date, repeat, search_from) - 1).max(0);
    let mut out = vec![];
    loop {
        let dates = period_dates(shape.start_date, repeat, p)?;
        let Some((_, first)) = dates.first() else {
            p += 1;
            continue;
        };
        if *first > frame_end {
            break;
        }
        let mut ended = false;
        for (index, date) in dates {
            let within = match &repeat.end {
                EventRepeatEnd::Never => true,
                EventRepeatEnd::Until(until) => date <= *until,
                EventRepeatEnd::Count(c) => index < *c as u64,
            };
            if !within {
                ended = true;
                break;
            }
            let o = make_occurrence(&shape, index, date)?;
            if o.overlaps(window) {
                out.push(o);
            }
        }
        if ended {
            break;
        }
        p += 1;
    }
    return Ok(out);
}

pub fn event_envelope(event: &MessageBlockEvent) -> Result<(Timestamp, Option<Timestamp>), String> {
    let shape = shape(&event.range)?;
    let first = make_occurrence(&shape, 0, shape.start_date)?;
    let widen_start = |o: &Occurrence| -> Result<Timestamp, String> {
        match o {
            Occurrence::AllDay { start, .. } => return Ok(
                utc_midnight(*start)?.checked_sub(TZ_MARGIN).map_err(|e| e.to_string())?,
            ),
            Occurrence::Timed { start, .. } => return Ok(*start),
        }
    };
    let widen_end = |o: &Occurrence| -> Result<Timestamp, String> {
        match o {
            Occurrence::AllDay { end, .. } => return Ok(
                utc_midnight(add_days(*end, 1)?)?.checked_add(TZ_MARGIN).map_err(|e| e.to_string())?,
            ),
            Occurrence::Timed { end, .. } => return Ok(*end),
        }
    };
    let start = widen_start(&first)?;
    let Some(repeat) = &event.repeat else {
        return Ok((start, Some(widen_end(&first)?)));
    };
    match &repeat.end {
        EventRepeatEnd::Never => return Ok((start, None)),
        EventRepeatEnd::Until(until) => {
            let p = period_containing(shape.start_date, repeat, *until) + 1;
            let mut last = first;
            for (index, date) in period_dates(shape.start_date, repeat, p)? {
                last = make_occurrence(&shape, index, date)?;
            }
            return Ok((start, Some(widen_end(&last)?)));
        },
        EventRepeatEnd::Count(c) => {
            let target = *c as u64 - 1;
            let p = match &repeat.rule {
                EventRepeatRule::Weeks(days) => {
                    let offsets = sorted_weekday_offsets(days);
                    let start_offset = shape.start_date.weekday().to_monday_zero_offset() as i64;
                    let first_count = offsets.iter().filter(|o| **o >= start_offset).count() as u64;
                    if target < first_count {
                        0
                    } else {
                        1 + ((target - first_count) / offsets.len() as u64) as i64
                    }
                },
                EventRepeatRule::Days | EventRepeatRule::Months(_) | EventRepeatRule::Years => target as i64,
            };
            let mut last = first;
            for (index, date) in period_dates(shape.start_date, repeat, p)? {
                if index == target {
                    last = make_occurrence(&shape, index, date)?;
                }
            }
            return Ok((start, Some(widen_end(&last)?)));
        },
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::interface::shared::{
            EventRangeAllDay,
            EventRangeTimed,
        },
        jiff::civil::date,
    };

    fn ev(range: EventRange, repeat: Option<EventRepeat>) -> MessageBlockEvent {
        return MessageBlockEvent {
            text: String::new(),
            range: range,
            repeat: repeat,
        };
    }

    fn all_day(s: Date, e: Date) -> EventRange {
        return EventRange::AllDay(EventRangeAllDay {
            start: s,
            end: e,
        });
    }

    fn starts(occ: &[Occurrence]) -> Vec<Date> {
        return occ.iter().map(|o| match o {
            Occurrence::AllDay { start, .. } => *start,
            Occurrence::Timed { start, .. } => start.to_zoned(TimeZone::UTC).date(),
        }).collect();
    }

    #[test]
    fn single_all_day() {
        let e = ev(all_day(date(2026, 3, 30), date(2026, 4, 2)), None);
        let w = month_window_utc(2026, 4).unwrap();
        assert_eq!(starts(&event_occurrences(&e, &w).unwrap()), vec![date(2026, 3, 30)]);
        let w = month_window_utc(2026, 5).unwrap();
        assert!(event_occurrences(&e, &w).unwrap().is_empty());
        let w = CalendarWindow::local(date(2026, 4, 2), date(2026, 4, 3), &TimeZone::UTC).unwrap();
        assert_eq!(event_occurrences(&e, &w).unwrap().len(), 1);
        let w = CalendarWindow::local(date(2026, 4, 3), date(2026, 4, 4), &TimeZone::UTC).unwrap();
        assert!(event_occurrences(&e, &w).unwrap().is_empty());
    }

    #[test]
    fn weekly() {
        let e = ev(all_day(date(2026, 1, 2), date(2026, 1, 2)), Some(EventRepeat {
            interval: 2,
            rule: EventRepeatRule::Weeks(vec![EventWeekday::Monday, EventWeekday::Friday]),
            end: EventRepeatEnd::Count(5),
        }));
        let w = CalendarWindow::local(date(2025, 12, 1), date(2026, 3, 1), &TimeZone::UTC).unwrap();
        let occ = event_occurrences(&e, &w).unwrap();
        assert_eq!(
            starts(&occ),
            vec![date(2026, 1, 2), date(2026, 1, 12), date(2026, 1, 16), date(2026, 1, 26), date(2026, 1, 30)]
        );
        assert_eq!(occ.iter().map(|o| o.index()).collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);
        let (_, end) = event_envelope(&e).unwrap();
        assert!(end.unwrap() >= utc_midnight(date(2026, 1, 31)).unwrap());
    }

    #[test]
    fn monthly_clamped_far_future() {
        let e = ev(all_day(date(2000, 1, 31), date(2000, 1, 31)), Some(EventRepeat {
            interval: 1,
            rule: EventRepeatRule::Months(EventMonthly::DayOfMonth),
            end: EventRepeatEnd::Never,
        }));
        let w = month_window_utc(9000, 2).unwrap();
        assert_eq!(starts(&event_occurrences(&e, &w).unwrap()), vec![date(9000, 2, 28)]);
        let e = ev(all_day(date(2026, 11, 26), date(2026, 11, 26)), Some(EventRepeat {
            interval: 1,
            rule: EventRepeatRule::Months(EventMonthly::LastWeekday),
            end: EventRepeatEnd::Never,
        }));
        let w = month_window_utc(2026, 12).unwrap();
        assert_eq!(starts(&event_occurrences(&e, &w).unwrap()), vec![date(2026, 12, 31)]);
    }

    #[test]
    fn timed_dst() {
        let e = ev(EventRange::Timed(EventRangeTimed {
            start: date(2026, 3, 1).at(9, 0, 0, 0),
            end: date(2026, 3, 1).at(10, 0, 0, 0),
            tz: "America/New_York".to_string(),
        }), Some(EventRepeat {
            interval: 1,
            rule: EventRepeatRule::Days,
            end: EventRepeatEnd::Until(date(2026, 3, 10)),
        }));
        let w = month_window_utc(2026, 3).unwrap();
        let occ = event_occurrences(&e, &w).unwrap();
        assert_eq!(occ.len(), 10);
        let Occurrence::Timed { start, .. } = occ[9] else {
            panic!();
        };
        assert_eq!(start.to_zoned(TimeZone::get("America/New_York").unwrap()).hour(), 9);
        let (_, end) = event_envelope(&e).unwrap();
        assert!(end.unwrap() >= utc_midnight(date(2026, 3, 10)).unwrap());
    }
}
