//! # Control
//!
//! The infiniscroll deals with lots of shakey parts by maintaining a limited set
//! of "logical" state, and the rest is synced to that:
//!
//! * Anchor element - which element is the screen "anchor".
//!
//! * Anchor element offset - how the origin is offset from the anchor (fine scrolling,
//!   within a single element)
//!
//! * Origin alignment - how the screen relates to the anchor element.  0 means the top
//!   of the anchor is at the top of the screen, 0.5 means the middle of the anchor is
//!   in the middle of the screen, 1 means the bottom of the anchor is at the bottom of
//!   the screen.
//!
//! When the user scrolls the anchor element and offset change.
//!
//! When the user scrolls to the end/start, or the stop element is reached via
//! pulling, the alignment is changed.
//!
//! Human-initiated scrolling, resizing, etc. subsequently "shake" everything to
//! remove wrinkles, matching everything with the above values, triggering new
//! entry requests, etc.
//!
//! # Shake
//!
//! Shake has two parts
//!
//! 1. Coming up with logical element layout.  This uses the logical values above, plus
//!    the computed element heights (which we don't manage logically, since for sticky we
//!    need to leave it to the DOM).
//!
//! 2. Matching the view to that layout
//!
//! # Sticky
//!
//! When realized, a sticky element is like normal, relying on css to keep it on
//! screen.
//!
//! When it moves into reserve, the realized state is moved into an early/late feed
//! holding bucket, and the dom element remains in tree.  It stays even if the
//! reserve is dropped.
//!
//! When the reserve is consumed, if the next item is sticky, it's just moved out
//! of the sticky bucket.
//!
//! # Implementation notes
//!
//! ## Multi-feed stop status and sorting
//!
//! The real elements are guaranteed to be ordered and gapless for all the feeds.
//! Basically we compare the next element in each feed before realizing, and always
//! realize the nearest element.  This means that if a feed does not have a next
//! element to compare, we can't complete the comparison, so we can't realize new
//! elements.  In this case we request new elements and wait for them to arrive
//! before realizing further.
//!
//! If there really are no elements, the remaining elements in each feed reserve
//! can be realized in sorted order. We indicate this with the stop marker.
//!
//! There's a race condition when new elements are created while in the stop state
//! (i.e. transitioning back to non-stop, although depending on the situation it
//! may be a temporary un-stop where we don't change the marker).  We can't know if
//! more elements will be created, so when we receive new elements at the end we
//! realize them immediately, but others come from another feed after that they may
//! have an order earlier than the previously realized elements, so when we receive
//! new elements at the end they must always be sorted in.
//!
//! ## Single feed stop status with async creation
//!
//! If we receive a notification of a new element while in a state where we don't
//! need it (scrolled up), we discard it.  However, there's a race condition where
//! if we're requesting new elements, and a new element is created after the server
//! looks up the elements but before the client receives the response, the response
//! won't include the new element.  If the notification arrives before the
//! response, we may discard it,
//!
//! The infiniscroll takes care of this currently.  When we receive a notification,
//! or an end fill occurs but doesn't reach the end, we always request elements
//! after the last known element in the feed, so any missing elements will be
//! included.
use {
    crate::util::MoreMath,
    flowcontrol::shed,
    gloo::timers::callback::Timeout,
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::{
        Container,
        ContainerEntry,
        El,
        ObserveHandle,
        ResizeObserver,
        el,
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::{
            HashMap,
            VecDeque,
        },
        hash::Hash,
        rc::{
            Rc,
            Weak,
        },
        time::{
            Duration,
            Instant,
        },
    },
    wasm_bindgen::JsCast,
    web_sys::{
        CssStyleDeclaration,
        HtmlElement,
    },
};

const PX_PER_CM: f64 = 96. / 2.54;
const BUFFER: f64 = PX_PER_CM * 40.;
const MIN_RESERVE: usize = 50;
const MAX_RESERVE: usize = MIN_RESERVE + 2 * 50;

trait ElExt {
    fn offset_top(&self) -> f64;
    fn offset_height(&self) -> f64;
}

impl ElExt for El {
    fn offset_top(&self) -> f64 {
        return self.raw().dyn_ref::<HtmlElement>().unwrap().offset_top() as f64;
    }

    fn offset_height(&self) -> f64 {
        return self.raw().dyn_ref::<HtmlElement>().unwrap().offset_height() as f64;
    }
}

fn el_style(el: &El) -> CssStyleDeclaration {
    return el.raw().dyn_into::<HtmlElement>().unwrap().style();
}

fn set_el_hide(el: &El, hide: bool) {
    let el = el_style(el);
    if hide {
        el.set_property("display", "none").unwrap();
    } else {
        el.remove_property("display").unwrap();
    }
}

pub trait FeedIdTraits: 'static + Clone + std::fmt::Debug + PartialEq + Eq + PartialOrd + Hash { }

impl<T: 'static + Clone + std::fmt::Debug + PartialEq + Eq + PartialOrd + Hash> FeedIdTraits for T { }

pub trait Entry {
    type FeedId: Clone + PartialEq + Eq + PartialOrd + Hash;
    type Time: 'static + Clone + std::fmt::Debug + PartialEq + Eq + PartialOrd + Hash + Default;

    fn create_el(&self, pc: &mut ProcessingContext) -> El;
    fn time(&self) -> Self::Time;
}

struct EntryState<E: Entry> {
    feed_id: E::FeedId,
    entry: Rc<E>,
    entry_el: El,
    _entry_el_observe: ObserveHandle,
}

impl<E: Entry> ContainerEntry for EntryState<E> {
    fn el(&self) -> &El {
        return &self.entry_el;
    }
}

/// A data source for the inifiniscroller. When it gets requests for elements, it
/// must only call the parent `respond_` and `notify_` functions after the stack
/// unwinds (spawn or timer next tick).
///
/// The stop states of the feed are controlled by the feed when it calls `respond_`
/// methods.
pub trait Feed<E: Entry> {
    fn set_parent(&self, parent: WeakInfinite<E>);

    // Must return the element with id `id`, or else, at least one element before and
    // after time (if they exist)
    fn request_around(&self, eg: EventGraph, time: E::Time);
    fn request_before(&self, eg: EventGraph, time: E::Time);
    fn request_after(&self, eg: EventGraph, time: E::Time);
}

struct FeedState<E: Entry> {
    feed: Box<dyn Feed<E>>,
    /// No elements, shortcut for request_around for initial data
    initial: bool,
    /// All entries are sorted and come before all realized entries. Front = nearest to
    /// real = late to early.
    early_reserve: VecDeque<Rc<E>>,
    /// All entries are sorted and come after all realized entries. Front = nearest to
    /// real = early to late.
    late_reserve: VecDeque<Rc<E>>,
    early_stop: bool,
    late_stop: bool,
    earliest_known: Option<E::Time>,
}

impl<E: Entry> FeedState<E> {
    fn update_earliest_known(&mut self, id: E::Time) {
        match &self.earliest_known {
            Some(i) => if id < *i {
                self.earliest_known = Some(id);
            },
            None => self.earliest_known = Some(id),
        }
    }
}

struct Infiniscroll_<E: Entry> {
    eg: EventGraph,
    /// Used when new/resetting
    reset_time: RefCell<E::Time>,
    padding_pre: El,
    padding_post: El,
    outer_stack: El,
    scroll_outer: El,
    cached_frame_height: Cell<f64>,
    content: El,
    content_layout: El,
    /// Mirrors content's height, used to avoid js round trips (keep in sync)
    logical_content_height: Cell<f64>,
    logical_content_layout_offset: Cell<f64>,
    logical_scroll_top: Cell<f64>,
    center_spinner: El,
    early_spinner: El,
    late_spinner: El,
    feeds: RefCell<HashMap<E::FeedId, FeedState<E>>>,
    want_sticky: RefCell<Option<E::Time>>,
    reserve_sticky_entry: RefCell<Option<EntryState<E>>>,
    early_sticky: El,
    late_sticky: El,
    /// All entries are sorted.
    real: RefCell<Container<EntryState<E>>>,
    cached_real_offset: Cell<f64>,
    /// None if real is empty (i.e. invalid index)
    anchor_i: Cell<Option<usize>>,
    anchor_alignment: Cell<f64>,
    /// Offset of anchor element origin from view (scrolling)/desired content
    /// (recentering) origin.  If alignment is 0 (origin is top of element), has range
    /// `-height..0` because if the element is below the origin the anchor would
    /// actually be the previous element. If alignment is 1, has range `0..height`.
    anchor_offset: Cell<f64>,
    shake_future: RefCell<Option<Timeout>>,
    entry_resize_observer: RefCell<Option<ResizeObserver>>,
    // After making content layout changes, the next scroll event will be synthetic
    // (not human-volitional), so ignore it for anchor modification.
    mute_scroll: Cell<Instant>,
    // After human-volitional scrolling, more scrolling may soon come so push back
    // shake for this number of ms.
    delay_shake: Cell<u32>,
}

fn calc_anchor_offset(real_origin_y: f64, anchor_top: f64, anchor_height: f64, anchor_alignment: f64) -> f64 {
    let anchor_origin_y = anchor_top + anchor_height * anchor_alignment;
    let anchor_offset = anchor_origin_y - real_origin_y;
    return anchor_offset;
}

impl<E: Entry> Infiniscroll_<E> {
    fn reanchor_inner(&self, mut anchor_i: usize, real_origin_y: f64) {
        // Move anchor pointer down until directly after desired element
        while let Some(e_state) = self.real.borrow().get(anchor_i + 1) {
            if e_state.entry_el.offset_top() > real_origin_y {
                break;
            }
            anchor_i += 1;
        }

        // Move anchor pointer up until directly above (=at) desired element.
        while let Some(e_state) = self.real.borrow().get(anchor_i) {
            if e_state.entry_el.offset_top() <= real_origin_y {
                break;
            }
            if anchor_i == 0 {
                break;
            }
            anchor_i -= 1;
        }

        // Calculate offset
        {
            let real = self.real.borrow();
            let anchor = real.get(anchor_i).unwrap();
            self
                .anchor_offset
                .set(
                    calc_anchor_offset(
                        real_origin_y,
                        anchor.entry_el.offset_top(),
                        anchor.entry_el.offset_height(),
                        self.anchor_alignment.get(),
                    ),
                );
        }

        // .
        self.anchor_i.set(Some(anchor_i));
    }

    fn scroll_reanchor(&self) {
        if let Some(anchor_i) = self.anchor_i.get() {
            let real_origin_y = 
                // Origin in content space
                self.logical_scroll_top.get() + self.anchor_alignment.get().mix(0., self.cached_frame_height.get())
                // Origin in content-layout space
                - self.logical_content_layout_offset.get() - self.cached_real_offset.get();
            self.reanchor_inner(anchor_i, real_origin_y);
        } else {
            self.anchor_i.set(None);
            self.anchor_offset.set(0.);
        }
    }

    // Change anchor based on logical values (anchor, alignment), + frame height
    fn transition_alignment_reanchor(&self) {
        let Some(anchor_i) = self.anchor_i.get() else {
            return;
        };
        let real_origin_y = {
            let real = self.real.borrow();
            let anchor = real.get(anchor_i).unwrap();
            anchor.entry_el.offset_top() + anchor.entry_el.offset_height() * self.anchor_alignment.get() -
                self.anchor_offset.get()
        };
        let candidate_early_real_origin_y =
            real_origin_y - self.cached_frame_height.get() * self.anchor_alignment.get();
        let candidate_late_real_origin_y =
            real_origin_y + self.cached_frame_height.get() * (1. - self.anchor_alignment.get());
        let mut early_all_stop = true;
        let mut late_all_stop = true;
        for f in self.feeds.borrow().values() {
            early_all_stop = early_all_stop && f.early_stop && f.early_reserve.is_empty();
            late_all_stop = late_all_stop && f.late_stop && f.late_reserve.is_empty();
        }
        let last_el = self.real.borrow().last().unwrap().entry_el.clone();
        let last_el_top = last_el.offset_top();
        let first_el = self.real.borrow().first().unwrap().entry_el.clone();
        let first_el_top = 0.;
        let first_el_height = first_el.offset_height();
        let first_el_bottom = first_el_top + first_el_height;

        // # Hovering late end, align to late end
        if late_all_stop && candidate_late_real_origin_y >= last_el_top {
            self.anchor_alignment.set(1.);
            self.anchor_i.set(Some(self.real.borrow().len() - 1));
            self
                .anchor_offset
                .set(
                    calc_anchor_offset(
                        candidate_late_real_origin_y,
                        last_el_top,
                        last_el.offset_height(),
                        self.anchor_alignment.get(),
                    ),
                );
            return;
        }

        // # Hovering early end, align to early end
        if early_all_stop && candidate_early_real_origin_y <= first_el_bottom {
            self.anchor_alignment.set(0.);
            self.anchor_i.set(Some(0));
            self
                .anchor_offset
                .set(
                    calc_anchor_offset(
                        candidate_early_real_origin_y,
                        first_el_top,
                        first_el_height,
                        self.anchor_alignment.get(),
                    ),
                );
            return;
        }

        // # Otherwise, revert to middle
        self.anchor_alignment.set(0.5);
        let new_real_origin_y = (candidate_early_real_origin_y + candidate_late_real_origin_y) / 2.;
        self.reanchor_inner(anchor_i, new_real_origin_y);
    }
}

fn get_pivot_early<
    E: Entry,
>(entries: &Container<EntryState<E>>, feed_id: &E::FeedId, f_state: &FeedState<E>) -> Option<E::Time> {
    return f_state
        .early_reserve
        .back()
        .map(|entry| entry.time())
        .or_else(
            || entries.iter().filter(|entry| &entry.feed_id == feed_id).map(|e_state| e_state.entry.time()).next(),
        )
        .or_else(|| f_state.late_reserve.front().map(|entry| entry.time()));
}

fn get_pivot_late<
    E: Entry,
>(entries: &Container<EntryState<E>>, feed_id: &E::FeedId, f_state: &FeedState<E>) -> Option<E::Time> {
    return f_state
        .late_reserve
        .back()
        .map(|entry| entry.time())
        .or_else(
            || entries
                .iter()
                .rev()
                .filter(|entry| &entry.feed_id == feed_id)
                .map(|e_state| e_state.entry.time())
                .next(),
        )
        .or_else(|| f_state.early_reserve.front().map(|entry| entry.time()));
}

fn realize_entry<
    E: Entry,
>(
    pc: &mut ProcessingContext,
    entry_resize_observer: &ResizeObserver,
    feed_id: &E::FeedId,
    entry: Rc<E>,
) -> EntryState<E> {
    let entry_el = entry.create_el(pc);
    return EntryState {
        feed_id: feed_id.clone(),
        entry: entry,
        entry_el: entry_el.clone(),
        _entry_el_observe: entry_resize_observer.observe(&entry_el.raw()),
    };
}

pub struct WeakInfinite<E: Entry>(Weak<Infiniscroll_<E>>);

impl<E: Entry> Clone for WeakInfinite<E> {
    fn clone(&self) -> Self {
        return Self(self.0.clone());
    }
}

impl<E: Entry> WeakInfinite<E> {
    pub fn upgrade(&self) -> Option<Infinite<E>> {
        return self.0.upgrade().map(Infinite);
    }
}

pub struct Infinite<E: Entry>(Rc<Infiniscroll_<E>>);

impl<E: Entry> Clone for Infinite<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct InfiniteEls {
    pub center_spinner: El,
    pub early_spinner: El,
    pub late_spinner: El,
}

impl<E: 'static + Entry> Infinite<E> {
    pub fn new(eg: &EventGraph, els: InfiniteEls) -> Self {
        let outer_stack = el("div").classes(&["infinite"]);
        {
            let s = el_style(&outer_stack);
            s.set_property("flex-grow", "1").unwrap();
            s.set_property("display", "grid").unwrap();
            s.set_property("grid-template-rows", "1fr").unwrap();
            s.set_property("max-height", "100%").unwrap();
            s.set_property("max-width", "100%").unwrap();
            s.set_property("justify-content", "stretch").unwrap();
            s.set_property("align-items", "stretch").unwrap();
        }
        let scroll_outer = el("div").classes(&["scroll_outer"]);
        {
            let s = el_style(&scroll_outer);
            s.set_property("flex-grow", "1").unwrap();
            s.set_property("overflow-y", "scroll").unwrap();
            s.set_property("scrollbar-width", "none").unwrap();
            s.set_property("max-height", "100%").unwrap();
            s.set_property("pointer-events", "initial").unwrap();
            //. s.set_property("transition", "padding-top 0.2s ease-out, padding-bottom 0.2s ease-out").unwrap();
        }

        fn style_padding(e: El) -> El {
            let s = el_style(&e);
            s.set_property("display", "flex").unwrap();
            s.set_property("flex-direction", "column").unwrap();
            return e;
        }

        let padding_pre = style_padding(el("div"));
        let padding_post = style_padding(el("div"));
        let scroll_inner = el("div").classes(&["scroll_inner"]);
        {
            let s = el_style(&scroll_inner);
            s.set_property("display", "flex").unwrap();
            s.set_property("flex-direction", "column").unwrap();
            s.set_property("position", "relative").unwrap();
        }
        let content_layout = el("div");
        {
            let s = el_style(&content_layout);
            s.set_property("position", "absolute").unwrap();
            s.set_property("display", "flex").unwrap();
            s.set_property("flex-direction", "column").unwrap();
        }

        fn style_sticky(e: El) -> El {
            e.ref_classes(&["real"]);
            let s = el_style(&e);
            s.set_property("position", "sticky").unwrap();
            s.set_property("top", "0px").unwrap();
            s.set_property("bottom", "0px").unwrap();
            s.set_property("display", "flex").unwrap();
            s.set_property("flex-direction", "column").unwrap();
            return e;
        }

        let content_lines_early_sticky = style_sticky(el("div"));
        let content_lines_real = Container::new({
            let e = el("div").classes(&["real"]);
            let s = el_style(&e);
            s.set_property("display", "flex").unwrap();
            s.set_property("flex-direction", "column").unwrap();
            e
        });
        let content_lines_late_sticky = style_sticky(el("div"));

        fn wrap_spinner(e: El) -> El {
            let out = el("div").push(e);
            let s = el_style(&out);
            s.set_property("display", "flex").unwrap();
            s.set_property("justify-content", "center").unwrap();
            s.set_property("align-items", "center").unwrap();
            return out;
        }

        let center_spinner = wrap_spinner(els.center_spinner);
        let early_spinner = wrap_spinner(els.early_spinner);
        let late_spinner = wrap_spinner(els.late_spinner);
        set_el_hide(&early_spinner, true);
        set_el_hide(&late_spinner, true);

        fn set_el_col(e: El, col: &str) -> El {
            let s = el_style(&e);
            s.set_property("grid-column", col).unwrap();
            s.set_property("grid-row", col).unwrap();
            return e;
        }

        outer_stack.ref_extend(
            vec![
                set_el_col(scroll_outer.clone(), "1"),
                set_el_col(
                    el("div").extend(vec![padding_pre.clone(), center_spinner.clone(), padding_post.clone()]),
                    "1",
                )
            ],
        );
        scroll_outer.ref_push(scroll_inner.clone());
        scroll_inner.ref_push(content_layout.clone());
        content_layout.ref_extend(
            vec![
                early_spinner.clone(),
                content_lines_early_sticky.clone(),
                content_lines_real.el().clone(),
                content_lines_late_sticky.clone(),
                late_spinner.clone()
            ],
        );
        let state = Infinite(Rc::new(Infiniscroll_ {
            eg: eg.clone(),
            reset_time: Default::default(),
            outer_stack: outer_stack,
            scroll_outer: scroll_outer.clone(),
            cached_frame_height: Cell::new(0.),
            padding_pre: padding_pre.clone(),
            padding_post: padding_post.clone(),
            content: scroll_inner.clone(),
            content_layout: content_layout,
            logical_content_height: Cell::new(0.),
            logical_content_layout_offset: Cell::new(0.),
            logical_scroll_top: Cell::new(0.),
            center_spinner: center_spinner,
            early_spinner: early_spinner,
            late_spinner: late_spinner,
            feeds: RefCell::new(HashMap::new()),
            want_sticky: RefCell::new(None),
            reserve_sticky_entry: RefCell::new(None),
            early_sticky: content_lines_early_sticky,
            late_sticky: content_lines_late_sticky,
            real: RefCell::new(content_lines_real),
            cached_real_offset: Cell::new(0.),
            anchor_i: Cell::new(None),
            anchor_alignment: Cell::new(0.5),
            anchor_offset: Cell::new(0.),
            shake_future: RefCell::new(None),
            entry_resize_observer: RefCell::new(None),
            mute_scroll: Cell::new(Instant::now() + Duration::from_millis(300)),
            delay_shake: Cell::new(0),
        }));
        {
            *state.0.entry_resize_observer.borrow_mut() = Some(ResizeObserver::new({
                let state = state.weak();
                move |_| {
                    let Some(state) = state.upgrade() else {
                        return;
                    };
                    state.0.mute_scroll.set(Instant::now() + Duration::from_millis(50));
                    state.shake();
                }
            }));
        }
        scroll_outer.ref_on("scroll", {
            let state = state.weak();
            move |_event| {
                let Some(state) = state.upgrade() else {
                    return;
                };
                if state.0.mute_scroll.get() >= Instant::now() {
                    return;
                }
                state.0.logical_scroll_top.set(state.0.scroll_outer.raw().scroll_top() as f64);
                state.0.scroll_reanchor();
                state.0.transition_alignment_reanchor();
                state.0.delay_shake.set(200);
                state.shake();
            }
        });
        scroll_outer.ref_on_resize({
            // Frame height change
            let state = state.weak();
            move |_, _, frame_height| {
                let Some(state) = state.upgrade() else {
                    return;
                };
                if frame_height == state.0.cached_frame_height.get() {
                    return;
                }
                state.0.cached_frame_height.set(frame_height);
                state.0.mute_scroll.set(Instant::now() + Duration::from_millis(50));
                state.shake();
            }
        });
        scroll_inner.ref_on_resize({
            // Content height change
            let state = state.weak();
            let old_content_height = Cell::new(-1.0f64);
            move |_, _, content_height| {
                let Some(state) = state.upgrade() else {
                    return;
                };
                if content_height == old_content_height.get() {
                    return;
                }
                old_content_height.set(content_height);
                state.0.scroll_outer.raw().set_scroll_top(state.0.logical_scroll_top.get().round() as i32);
                state.0.mute_scroll.set(Instant::now() + Duration::from_millis(50));
            }
        });
        padding_pre.ref_on_resize({
            let state = state.weak();
            move |_, _w, h| {
                let Some(state) = state.upgrade() else {
                    return;
                };
                state
                    .0
                    .scroll_outer
                    .raw()
                    .dyn_ref::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("padding-top", &h.to_string())
                    .unwrap();
            }
        });
        padding_post.ref_on_resize({
            let state = state.weak();
            move |_, _w, h| {
                let Some(state) = state.upgrade() else {
                    return;
                };
                state
                    .0
                    .scroll_outer
                    .raw()
                    .dyn_ref::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("padding-bottom", &h.to_string())
                    .unwrap();
            }
        });
        return state;
    }

    pub fn add_feed(&self, id: E::FeedId, feed: impl 'static + Feed<E>) {
        feed.set_parent(self.weak());
        self.0.feeds.borrow_mut().insert(id, FeedState {
            feed: Box::new(feed),
            initial: true,
            early_reserve: VecDeque::new(),
            late_reserve: VecDeque::new(),
            early_stop: false,
            late_stop: false,
            earliest_known: None,
        });
        self.shake();
    }

    pub fn weak(&self) -> WeakInfinite<E> {
        return WeakInfinite(Rc::downgrade(&self.0));
    }

    pub fn el(&self) -> El {
        return self.0.outer_stack.clone();
    }

    pub fn padding_pre_el(&self) -> El {
        return self.0.padding_pre.clone();
    }

    pub fn padding_post_el(&self) -> El {
        return self.0.padding_post.clone();
    }

    fn jump_to_(&self, time: &E::Time) {
        *self.0.reset_time.borrow_mut() = time.clone();
        self.0.real.borrow_mut().clear();
        self.0.anchor_i.set(None);
        self.0.anchor_alignment.set(0.5);
        self.0.anchor_offset.set(0.);
        for f in self.0.feeds.borrow_mut().values_mut() {
            f.early_reserve.clear();
            f.late_reserve.clear();
            f.early_stop = false;
            f.late_stop = false;
            f.initial = true;
            f.earliest_known = None;
        }
    }

    pub fn jump_to(&self, time: &E::Time) {
        self.jump_to_(time);
        self.shake_immediate();
    }

    pub fn set_sticky(&self, id: &E::Time) {
        let mut changed = false;
        shed!{
            'done _;
            if let Some(have_id) = self.0.want_sticky.borrow().as_ref() {
                if have_id == id {
                    // No change
                    break 'done;
                } else {
                    // Changed; clean up old sticky entry
                    if let Some(f) = self.0.reserve_sticky_entry.borrow().as_ref() {
                        changed = true;
                        f.entry_el.ref_replace(vec![]);
                    }
                }
            }
            *self.0.want_sticky.borrow_mut() = Some(id.clone());
            // Locate rendered
            for e in &*self.0.real.borrow() {
                if &e.entry.time() == id {
                    // Element exists, must be marked sticky but otherwise all good
                    break 'done;
                }
            }
            // Not rendered; clear and jump
            changed = true;
            self.jump_to_(id);
        }
        if changed {
            self.shake_immediate();
        }
    }

    pub fn clear_sticky(&self) {
        *self.0.want_sticky.borrow_mut() = None;
        let mut changed = false;
        if let Some(s) = self.0.reserve_sticky_entry.borrow_mut().take() {
            changed = true;
            s.entry_el.ref_replace(vec![]);
        }
        if changed {
            self.shake();
        }
    }

    fn shake_immediate(&self) {
        let eg = self.0.eg.clone();
        eg.event(|pc| {
            self.0.delay_shake.set(0);
            *self.0.shake_future.borrow_mut() = None;

            // # Calculate content + current theoretical used space
            let mut used_early = 0f64;
            let mut used_late = 0f64;
            let mut real_origin_y = 0f64;
            if !self.0.real.borrow().is_empty() {
                let real = self.0.real.borrow_mut();
                let real_height = real.el().offset_height();
                let anchor_i = self.0.anchor_i.get().unwrap();
                let anchor = &mut real.get(anchor_i).unwrap();
                let anchor_top = anchor.entry_el.offset_top();
                let anchor_height = anchor.entry_el.offset_height();
                real_origin_y = anchor_top + anchor_height * self.0.anchor_alignment.get()
                    // Shift up becomes early usage
                    - self.0.anchor_offset.get();
                used_early = real_origin_y;
                used_late = real_height - real_origin_y;
            }

            // # Realize and unrealize elements to match goal bounds
            //
            // ## Early...
            let want_nostop_early = BUFFER + self.0.cached_frame_height.get() * self.0.anchor_alignment.get();
            let mut unrealize_early = 0usize;
            for e in &*self.0.real.borrow() {
                let bottom = e.entry_el.offset_top() + e.entry_el.offset_height();
                let min_dist = real_origin_y - bottom;
                if min_dist <= want_nostop_early {
                    break;
                }
                unrealize_early += 1;
                used_early = real_origin_y - bottom;
            }
            let mut stop_all_early = true;
            let mut realized_early = vec![];
            shed!{
                'realize_early _;
                while used_early < want_nostop_early {
                    let mut use_feed = None;
                    for (feed_id, f_state) in &*self.0.feeds.borrow() {
                        let Some(entry) = f_state.early_reserve.front() else {
                            // Reserve empty
                            if f_state.early_stop {
                                continue;
                            } else {
                                // Pending more
                                stop_all_early = false;
                                break 'realize_early;
                            }
                        };
                        let replace = match &use_feed {
                            Some((_, time)) => {
                                entry.time() > *time
                            },
                            None => {
                                true
                            },
                        };
                        if replace {
                            use_feed = Some((feed_id.clone(), entry.time()));
                        }
                    }
                    let Some((feed_id, _)) = use_feed else {
                        break 'realize_early;
                    };
                    let entry =
                        self.0.feeds.borrow_mut().get_mut(&feed_id).unwrap().early_reserve.pop_front().unwrap();
                    let mut real = None;
                    {
                        let reserve_sticky_entry = self.0.reserve_sticky_entry.borrow_mut();
                        if let Some(f) = &*reserve_sticky_entry {
                            if f.entry.time() == entry.time() {
                                let real1 = self.0.reserve_sticky_entry.borrow_mut().take().unwrap();
                                real1.entry_el.ref_replace(vec![]);
                                real = Some(real1);
                            }
                        }
                    }
                    let real =
                        real.unwrap_or_else(
                            || realize_entry(
                                pc,
                                self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                &feed_id,
                                entry,
                            ),
                        );
                    self.0.real.borrow_mut().el().ref_push(real.entry_el.clone());
                    let height = real.entry_el.offset_height();
                    real.entry_el.ref_replace(vec![]);
                    used_early += height;
                    realized_early.push(real);
                }
                stop_all_early = false;
            };

            // ## Late...
            let want_nostop_late = BUFFER + self.0.cached_frame_height.get() * (1. - self.0.anchor_alignment.get());
            let mut unrealize_late = 0usize;
            for e in self.0.real.borrow().iter().rev() {
                let top = e.entry_el.offset_top();
                let min_dist = top - real_origin_y;
                if min_dist <= want_nostop_late {
                    break;
                }
                unrealize_late += 1;
                used_late = top - real_origin_y;
            }
            let mut stop_all_late = true;
            let mut realized_late = vec![];
            shed!{
                'realize_late _;
                while used_late < want_nostop_late {
                    let mut use_feed = None;
                    for (feed_id, f_state) in &*self.0.feeds.borrow() {
                        let Some(entry) = f_state.late_reserve.front() else {
                            // Reserve empty
                            if f_state.late_stop {
                                continue;
                            } else {
                                // Pending more
                                stop_all_late = false;
                                break 'realize_late;
                            }
                        };
                        let replace = match &use_feed {
                            Some((_, time)) => {
                                entry.time() < *time
                            },
                            None => {
                                true
                            },
                        };
                        if replace {
                            use_feed = Some((feed_id.clone(), entry.time()));
                        }
                    }
                    let Some((feed_id, _)) = use_feed else {
                        break 'realize_late;
                    };
                    let entry =
                        self.0.feeds.borrow_mut().get_mut(&feed_id).unwrap().late_reserve.pop_front().unwrap();
                    let mut real = None;
                    if let Some(f) = &*self.0.reserve_sticky_entry.borrow() {
                        if f.entry.time() == entry.time() {
                            let real1 = self.0.reserve_sticky_entry.take().unwrap();
                            real1.entry_el.ref_replace(vec![]);
                            real = Some(real1);
                        }
                    }
                    let real =
                        real.unwrap_or_else(
                            || realize_entry(
                                pc,
                                self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                &feed_id,
                                entry,
                            ),
                        );
                    self.0.content.ref_push(real.entry_el.clone());
                    let height = real.entry_el.offset_height();
                    real.entry_el.ref_replace(vec![]);
                    used_late += height;
                    realized_late.push(real);
                }
                stop_all_late = false;
            };

            // ## Apply changes
            //
            // ### Update anchor
            match self.0.anchor_i.get() {
                Some(anchor_i) => {
                    self.0.anchor_i.set(Some(anchor_i + realized_early.len() - unrealize_early));
                },
                None => {
                    match (realized_early.is_empty(), realized_late.is_empty()) {
                        (true, true) => {
                            // nop
                        },
                        (true, false) => {
                            self.0.anchor_i.set(Some(0));
                        },
                        (false, _) => {
                            self.0.anchor_i.set(Some(realized_early.len() - 1));
                        },
                    }
                },
            }

            // ### New early elements
            //
            // late to early -> early to late
            realized_early.reverse();
            for evicted_e_state in self.0.real.borrow_mut().splice(0, unrealize_early, realized_early) {
                let mut feeds = self.0.feeds.borrow_mut();
                let feed = feeds.get_mut(&evicted_e_state.feed_id).unwrap();
                feed.early_reserve.push_front(evicted_e_state.entry.clone());
                if self.0.want_sticky.borrow().iter().any(|s| s == &evicted_e_state.entry.time()) {
                    self.0.early_sticky.ref_push(evicted_e_state.el().clone());
                    *self.0.reserve_sticky_entry.borrow_mut() = Some(evicted_e_state);
                }
            }

            // ### Late elements
            for evicted_e_state in self
                .0
                .real
                .borrow_mut()
                .splice(self.0.real.borrow().len() - unrealize_late, unrealize_late, realized_late)
                .rev() {
                let mut feeds = self.0.feeds.borrow_mut();
                let feed = feeds.get_mut(&evicted_e_state.feed_id).unwrap();
                feed.late_reserve.push_front(evicted_e_state.entry.clone());
                if self.0.want_sticky.borrow().iter().any(|s| s == &evicted_e_state.entry.time()) {
                    self.0.late_sticky.ref_push(evicted_e_state.el().clone());
                    *self.0.reserve_sticky_entry.borrow_mut() = Some(evicted_e_state);
                }
            }

            // # Prune reserve and unset stop status
            let mut requesting_early = false;
            let mut requesting_late = false;
            let center = {
                let entries = self.0.real.borrow();
                entries.get(entries.len() / 2).map(|x| x.entry.time())
            };
            for (feed_id, f_state) in &mut *self.0.feeds.borrow_mut() {
                if f_state.initial {
                    f_state
                        .feed
                        .request_around(
                            pc.eg(),
                            center.as_ref().cloned().unwrap_or_else(|| self.0.reset_time.borrow().clone()),
                        );
                    requesting_early = true;
                    requesting_late = true;
                } else {
                    if f_state.early_reserve.len() > MAX_RESERVE {
                        f_state.early_reserve.truncate(MAX_RESERVE);
                        f_state.early_stop = false;
                    }
                    if !f_state.early_stop && f_state.early_reserve.len() < MIN_RESERVE {
                        let pivot = get_pivot_early(&*self.0.real.borrow(), feed_id, f_state).unwrap();
                        f_state.feed.request_before(pc.eg(), pivot);
                        requesting_early = true;
                    }
                    if f_state.late_reserve.len() > MAX_RESERVE {
                        f_state.late_reserve.truncate(MAX_RESERVE);
                        f_state.late_stop = false;
                    }
                    if !f_state.late_stop && f_state.late_reserve.len() < MIN_RESERVE {
                        let pivot = get_pivot_late(&*self.0.real.borrow(), feed_id, f_state).unwrap();
                        f_state.feed.request_after(pc.eg(), pivot);
                        requesting_late = true;
                    }
                }
            }
            set_el_hide(&self.0.center_spinner, !(self.0.real.borrow().is_empty() && requesting_early));
            set_el_hide(&self.0.early_spinner, !(!self.0.real.borrow().is_empty() && requesting_early));
            set_el_hide(&self.0.late_spinner, !(!self.0.real.borrow().is_empty() && requesting_late));
            self.0.cached_real_offset.set(self.0.real.borrow().el().offset_top());

            // # Update alignment based on used space, stop states
            self.0.transition_alignment_reanchor();

            // # Calculate desired space per used space + stop status
            //
            // Distance from content-origin to start of content
            let want_early;
            if stop_all_early {
                want_early = used_early.min(want_nostop_early);
            } else {
                want_early = want_nostop_early;
            }

            // Distance from content-origin to end of content
            let want_late;
            if stop_all_late {
                want_late = used_late.min(want_nostop_late);
            } else {
                want_late = want_nostop_late;
            }

            // # Update logical height, deferred real height update
            let new_height = (want_early + want_late).max(self.0.cached_frame_height.get());
            if (new_height - self.0.logical_content_height.get()).abs() >= 1. {
                self.0.logical_content_height.set(new_height);
                self
                    .0
                    .content
                    .raw()
                    .dyn_ref::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("height", &format!("{}px", new_height))
                    .unwrap();
            }

            // # Position content based on content size, used space, and alignment
            self.0.logical_content_layout_offset.set(self.0.anchor_alignment.get().mix(
                // Start of content is origin (want_early) minus the amount used before
                // (used_early)
                want_early - used_early,
                // Backwards from end of content, in case used < frame height.
                // `logical_content_height` is padded, so this will push it to the end.
                self.0.logical_content_height.get() - (want_late + used_early),
            ) - self.0.cached_real_offset.get());
            self
                .0
                .content_layout
                .raw()
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .style()
                .set_property("top", &format!("{}px", self.0.logical_content_layout_offset.get()))
                .unwrap();

            // # Calculate centered scroll so visual origin matches content origin
            self
                .0
                .logical_scroll_top
                .set(
                    self
                        .0
                        .anchor_alignment
                        .get()
                        .mix(
                            want_early,
                            self.0.logical_content_height.get() - want_late - self.0.cached_frame_height.get(),
                        )
                        .max(0.),
                );
            self.0.scroll_outer.raw().set_scroll_top(self.0.logical_scroll_top.get().round() as i32);
            self.0.mute_scroll.set(Instant::now() + Duration::from_millis(50));
        });
    }

    fn shake(&self) {
        let mute_scroll = self.0.mute_scroll.get() >= Instant::now();
        if mute_scroll || self.0.delay_shake.get() == 0 {
            self.shake_immediate();
        } else {
            *self.0.shake_future.borrow_mut() = Some(Timeout::new(self.0.delay_shake.get(), {
                let state = self.weak();
                move || {
                    let Some(state) = state.upgrade() else {
                        return;
                    };
                    state.shake_immediate();
                }
            }));
        }
    }

    /// Called by feed, in response to `request__around`.
    pub fn respond_entries_around(
        &self,
        feed_id: &E::FeedId,
        pivot: &E::Time,
        entries: Vec<Rc<E>>,
        early_stop: bool,
        late_stop: bool,
    ) {
        self.0.eg.event(|pc| {
            if *pivot != *self.0.reset_time.borrow() {
                return;
            }
            {
                let mut feeds = self.0.feeds.borrow_mut();
                let feed = feeds.get_mut(&feed_id).unwrap();
                if !feed.initial {
                    return;
                }
                feed.initial = false;

                // early to late
                let mut prepend = vec![];

                // early to late
                let mut postpend = vec![];
                for e in entries {
                    let time = e.time();
                    feed.update_earliest_known(time.clone());
                    feed.update_earliest_known(time.clone());
                    if time < *self.0.reset_time.borrow() {
                        prepend.push(e);
                    } else if time == *self.0.reset_time.borrow() {
                        let real =
                            realize_entry(pc, self.0.entry_resize_observer.borrow().as_ref().unwrap(), feed_id, e);
                        self.0.real.borrow_mut().push(real);
                        self.0.anchor_i.set(Some(0));
                    } else {
                        postpend.push(e);
                    }
                }
                for e in &prepend {
                    if self.0.want_sticky.borrow().iter().any(|s| s == &e.time()) {
                        let real =
                            realize_entry(
                                pc,
                                self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                feed_id,
                                e.clone(),
                            );
                        self.0.early_sticky.ref_push(real.el().clone());
                        *self.0.reserve_sticky_entry.borrow_mut() = Some(real);
                    }
                }
                prepend.reverse();
                feed.early_reserve.extend(prepend);
                for e in &postpend {
                    if self.0.want_sticky.borrow().iter().any(|s| s == &e.time()) {
                        let real =
                            realize_entry(
                                pc,
                                self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                feed_id,
                                e.clone(),
                            );
                        self.0.late_sticky.ref_push(real.el().clone());
                        *self.0.reserve_sticky_entry.borrow_mut() = Some(real);
                    }
                }
                feed.late_reserve.extend(postpend);
                feed.early_stop = early_stop;
                feed.late_stop = late_stop;
            }
            self.0.transition_alignment_reanchor();
        });
        self.shake();
    }

    /// Called by feed in response to `request_before`.
    ///
    /// * `initial_pivot` is the pivot `request_before` was called with.
    ///
    /// * `entries` must be sorted latest to earliest (descending).
    ///
    /// * `stop` indicates whether the server knows of more entries past the most extreme
    ///   entry at the time the request was received.
    pub fn respond_entries_before(
        &self,
        feed_id: &E::FeedId,
        initial_pivot: &E::Time,
        entries: Vec<Rc<E>>,
        mut stop: bool,
    ) {
        if entries.is_empty() {
            return;
        }
        assert!(shed!{
            'assert _;
            // Assert properly ordered and >= pivot
            let mut at = initial_pivot.clone();
            for e in &entries {
                if e.time() >= at {
                    break 'assert false;
                }
                at = e.time();
            }
            true
        });
        {
            let Some(current_pivot) =
                get_pivot_early(&self.0.real.borrow(), feed_id, self.0.feeds.borrow().get(feed_id).unwrap()) else {
                    return;
                };
            if initial_pivot != &current_pivot {
                return;
            }
            self.0.eg.event(|pc| {
                {
                    let mut feeds = self.0.feeds.borrow_mut();
                    let feed_state = feeds.get_mut(feed_id).unwrap();
                    {
                        let earliest_known = feed_state.earliest_known.clone().unwrap();
                        if initial_pivot != &earliest_known && entries.iter().all(|e| e.time() != earliest_known) {
                            // Know of element beyond this result (via an async channel)
                            stop = false;
                        }
                    }
                    feed_state.update_earliest_known(entries.last().unwrap().time());
                    for e in entries.iter().rev() {
                        if self.0.want_sticky.borrow().iter().any(|s| s == &e.time()) {
                            let real =
                                realize_entry(
                                    pc,
                                    self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                    feed_id,
                                    e.clone(),
                                );
                            self.0.early_sticky.ref_push(real.el().clone());
                            *self.0.reserve_sticky_entry.borrow_mut() = Some(real);
                        }
                    }
                    feed_state.early_reserve.extend(entries);
                    feed_state.early_stop = stop;
                }
                self.0.transition_alignment_reanchor();
            });
        }
        self.shake();
    }

    /// Called by `feed` in response to `request_after`.
    ///
    /// * `initial_pivot` - The pivot `request_after` was called with.
    ///
    /// * `entries` must be sorted earliest to latest.
    ///
    /// * `stop` indicates whether the server knows of more entries past the most extreme
    ///   entry at the time the request was received.
    pub fn respond_entries_after(
        &self,
        feed_id: &E::FeedId,
        initial_pivot: &E::Time,
        mut entries: Vec<Rc<E>>,
        mut stop: bool,
    ) {
        if entries.is_empty() {
            return;
        }
        assert!(shed!{
            'assert _;
            // Confirm sorting
            let mut at = initial_pivot.clone();
            for e in &entries {
                if e.time() <= at {
                    break 'assert false;
                }
                at = e.time();
            }
            true
        });
        {
            let Some(current_pivot) =
                get_pivot_late(&self.0.real.borrow(), feed_id, self.0.feeds.borrow().get(feed_id).unwrap()) else {
                    return;
                };
            if initial_pivot != &current_pivot {
                return;
            }
            self.0.eg.event(|pc| {
                // Gather overall state
                let mut all_stopped = true;
                let mut all_reserve_empty = true;
                for feed_state in self.0.feeds.borrow().values() {
                    if !feed_state.late_stop {
                        all_stopped = false;
                    }
                    if !feed_state.late_reserve.is_empty() {
                        all_reserve_empty = false;
                    }
                }
                {
                    let mut feeds = self.0.feeds.borrow_mut();
                    let feed_state = feeds.get_mut(feed_id).unwrap();
                    shed!{
                        'done_adding _;
                        macro_rules! add_to_reserve{
                            () => {
                                for entry in entries {
                                    if feed_state.late_reserve.len() < MAX_RESERVE {
                                        let entry_time = entry.time();
                                        if self.0.want_sticky.borrow().iter().any(|s| s == &entry_time) {
                                            let real =
                                                realize_entry(
                                                    pc,
                                                    self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                                    feed_id,
                                                    entry.clone(),
                                                );
                                            *self.0.reserve_sticky_entry.borrow_mut() = Some(real);
                                        }
                                        feed_state.late_reserve.push_back(entry);
                                    } else {
                                        stop = false;
                                        break;
                                    }
                                }
                            };
                        }
                        // Already a reserve, new elements guaranteed to be ordered afterwards
                        if !feed_state.late_reserve.is_empty() {
                            add_to_reserve!();
                            feed_state.late_stop = stop;
                            break 'done_adding;
                        }
                        // No reserve, new elements could need to be sorted into real depending on how
                        // feed events interleaved
                        let real_latest_time = self.0.real.borrow().last().unwrap().entry.time();
                        entries.reverse();
                        let mut last_insert_before_i = 0;
                        loop {
                            let Some(entry) = entries.last() else {
                                break;
                            };
                            let entry_time = entry.time();
                            if entry_time >= real_latest_time {
                                break;
                            }
                            let entry = entries.pop().unwrap();
                            let insert_before_i = shed!{
                                'find_insert _;
                                for (
                                    i,
                                    real_state,
                                ) in self.0.real.borrow().iter().enumerate().skip(last_insert_before_i).rev() {
                                    if entry_time > real_state.entry.time() {
                                        break 'find_insert i + 1;
                                    }
                                }
                                break 'find_insert 0;
                            };
                            last_insert_before_i = insert_before_i;
                            if insert_before_i == 0 {
                                // Insert at start of early reserve, because insertion is unbounded within
                                // realized elements (shake will realize it if necessary) OR no real elements
                                if self.0.want_sticky.borrow().iter().any(|s| s == &entry_time) {
                                    let real =
                                        realize_entry(
                                            pc,
                                            self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                            feed_id,
                                            entry.clone(),
                                        );
                                    self.0.early_sticky.ref_push(real.el().clone());
                                    *self.0.reserve_sticky_entry.borrow_mut() = Some(real);
                                }
                                feed_state.early_reserve.push_front(entry);
                            } else {
                                // Insert within real elements
                                let real = realize_entry(pc, self.0.entry_resize_observer.borrow().as_ref().unwrap(), feed_id, entry);
                                let anchor_i = self.0.anchor_i.get().unwrap();
                                if insert_before_i <= anchor_i {
                                    self.0.anchor_i.set(Some(anchor_i + 1));
                                }
                                self.0.real.borrow_mut().insert(insert_before_i, real);
                            }
                        }
                        // Remaining new elements come after the final real element
                        entries.reverse();
                        if all_stopped && all_reserve_empty {
                            // No other feeds have reserve so these are the guaranteed next (of known
                            // elements) - go ahead and realize.
                            for entry in entries {
                                let real =
                                    realize_entry(
                                        pc,
                                        self.0.entry_resize_observer.borrow().as_ref().unwrap(),
                                        feed_id,
                                        entry,
                                    );
                                let anchor_i = self.0.anchor_i.get().unwrap();
                                if anchor_i == self.0.real.borrow().len() - 1 {
                                    self.0.anchor_i.set(Some(anchor_i + 1));
                                }
                                self.0.real.borrow_mut().push(real);
                            }
                        }
                        else {
                            // Other feeds have reserve so these need to be pulled in by ordering (because
                            // there's reserve we can't be anchored at last element anyway)
                            add_to_reserve!();
                        }
                        feed_state.late_stop = stop;
                    };
                }
            });
        }
        self.shake();
    }

    /// Called by feed when notified of new entries, to decide if the view is in a
    /// state where it can accept more entries. Returns a pivot if new entries are
    /// acceptable.
    pub fn want_after(&self, feed_id: E::FeedId) -> Option<E::Time> {
        let mut feeds = self.0.feeds.borrow_mut();
        let f_state = feeds.get_mut(&feed_id).unwrap();
        if !f_state.late_stop {
            return None;
        }
        return get_pivot_late(&self.0.real.borrow(), &feed_id, f_state);
    }
}
