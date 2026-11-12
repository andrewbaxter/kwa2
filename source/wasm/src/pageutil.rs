use {
    crate::{
        api::{
            file_url,
            req_file_blob,
        },
        js::{
            copy_text,
            el_async,
            style_export,
        },
        localdata::{
            NowOrLater,
            req_api_channelgroups,
            req_api_identities,
        },
        state::{
            Ministate,
            ministate_octothorpe,
            spawn_rooted_log,
            state,
        },
    },
    flowcontrol::ta_return,
    js_sys::Math::random,
    lunk::{
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        el,
        scope_any,
        spawn_rooted,
    },
    rooting_forms::{
        css::err_el,
        impl_file::{
            initial_blob,
            FileFormState,
            FileValue,
        },
    },
    shared::interface::{
        shared::{
            ChannelGroupId,
            FileRef,
            Portrait,
        },
        wire::{
            c2s::{
                ChannelRetention,
                ModifyOption,
            },
            kwaurl::KwaUrl,
        },
    },
    spaghettinuum::byteszb32::BytesZb32,
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        rc::Rc,
    },
    qrcode::{
        QrCode,
        render::svg,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        HtmlInputElement,
        HtmlSelectElement,
    },
};

#[derive(Clone)]
pub struct FormPortrait(pub FileValue);

impl From<FileValue> for FormPortrait {
    fn from(value: FileValue) -> Self {
        return FormPortrait(value);
    }
}

pub async fn req_form_portrait(file: &Option<FileRef>) -> Result<FormPortrait, String> {
    match file {
        Some(f) => return Ok(
            FormPortrait(FileValue::Keep(Some(req_file_blob(&file_url(f), &f.mime_type).await?))),
        ),
        None => return Ok(FormPortrait(FileValue::Keep(None))),
    }
}

pub async fn read_portrait(file: web_sys::File) -> Result<Portrait, String> {
    let mime_type = file.type_();
    let data = gloo::file::futures::read_as_bytes(&gloo::file::File::from(file)).await.map_err(|e| e.to_string())?;
    return Ok(Portrait {
        mime_type: mime_type,
        data: BytesZb32(data),
    });
}

impl FormPortrait {
    pub async fn into_modify(self) -> Result<Option<ModifyOption<Portrait>>, String> {
        match self.0 {
            FileValue::Keep(_) => return Ok(None),
            FileValue::Clear => return Ok(Some(ModifyOption { value: None })),
            FileValue::Set(f) => return Ok(Some(ModifyOption { value: Some(read_portrait(f).await?) })),
        }
    }
}

impl<C: 'static + Clone> rooting_forms::FormWith<C> for FormPortrait {
    fn new_form_with_(
        _context: &C,
        field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (rooting_forms::FormElements, Box<dyn rooting_forms::FormState<Self>>) {
        return FileFormState::new(field, Some("image/*"), initial_blob(from.map(|x| &x.0)));
    }
}

pub struct FormIdentity(pub Identity);

struct FormIdentityState {
    err_el: El,
    select_el: Rc<RefCell<Option<El>>>,
}

impl rooting_forms::FormState<FormIdentity> for FormIdentityState {
    fn parse(&self) -> Result<FormIdentity, ()> {
        self.err_el.ref_clear();
        let select_el = self.select_el.borrow();
        let Some(select_el) = select_el.as_ref() else {
            self.err_el.ref_text("Identity not selected, still loading");
            return Err(());
        };
        let el_ = select_el.raw().dyn_into::<HtmlSelectElement>().unwrap();
        match serde_json::from_str(&el_.value()).map_err(|e| e.to_string()) {
            Ok(v) => return Ok(FormIdentity(v)),
            Err(e) => {
                self.err_el.ref_text(&e);
                return Err(());
            },
        }
    }
}

impl<C: 'static + Clone> rooting_forms::FormWith<C> for FormIdentity {
    fn new_form_with_(
        _context: &C,
        _field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (rooting_forms::FormElements, Box<dyn rooting_forms::FormState<Self>>) {
        let err_el = err_el();
        let select_el = Rc::new(RefCell::new(None));
        return (rooting_forms::FormElements {
            error: Some(err_el.clone()),
            elements: vec![el_async({
                let from = from.map(|x| x.0.clone());
                let select_el = select_el.clone();
                async move {
                    ta_return!(Vec < El >, String);
                    let out = el("select");
                    *select_el.borrow_mut() = Some(out.clone());
                    let mut identities = req_api_identities(None).await?;
                    identities.sort_by_cached_key(|x| (x.last_used, x.res.id.clone()));
                    for i in identities {
                        let value = serde_json::to_string(&i.res.id).unwrap();
                        out.ref_push(el("option").attr("value", &value).text(&i.res.name));
                        if let Some(s) = &from {
                            if *s == i.res.id {
                                out.ref_attr("value", &value);
                            }
                        }
                    }
                    return Ok(vec![out]);
                }
            })],
        }, Box::new(FormIdentityState {
            err_el: err_el,
            select_el: select_el,
        }))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct FormOptChannelGroup(pub Option<ChannelGroupId>);

struct FormOptChannelGroupState {
    err_el: El,
    select_el: Rc<RefCell<Option<El>>>,
}

impl rooting_forms::FormState<FormOptChannelGroup> for FormOptChannelGroupState {
    fn parse(&self) -> Result<FormOptChannelGroup, ()> {
        self.err_el.ref_clear();
        let select_el = self.select_el.borrow();
        let Some(select_el) = select_el.as_ref() else {
            self.err_el.ref_text("Channel group selection missing, still loading choices");
            return Err(());
        };
        let el_ = select_el.raw().dyn_into::<HtmlSelectElement>().unwrap();
        match serde_json::from_str(&el_.value()).map_err(|e| e.to_string()) {
            Ok(v) => return Ok(FormOptChannelGroup(v)),
            Err(e) => {
                self.err_el.ref_text(&e);
                return Err(());
            },
        }
    }
}

impl<C: 'static + Clone> rooting_forms::FormWith<C> for FormOptChannelGroup {
    fn new_form_with_(
        _context: &C,
        _field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (rooting_forms::FormElements, Box<dyn rooting_forms::FormState<Self>>) {
        let err_el = err_el();
        let select_el = Rc::new(RefCell::new(None));
        return (rooting_forms::FormElements {
            error: Some(err_el.clone()),
            elements: vec![el_async({
                let from = from.map(|x| x.0.clone());
                let select_el = select_el.clone();
                async move {
                    ta_return!(Vec < El >, String);
                    let out = el("select");
                    *select_el.borrow_mut() = Some(out.clone());
                    out.ref_push(
                        el("option")
                            .attr("value", &serde_json::to_string(&(None as Option<ChannelGroupId>)).unwrap())
                            .text("No group"),
                    );
                    let mut cgs = req_api_channelgroups(None).await?;
                    cgs.sort_by_cached_key(|x| (x.last_used, x.res.id.clone()));
                    for i in cgs {
                        let value = serde_json::to_string(&i.res.id).unwrap();
                        let option_el = el("option").attr("value", &value).text(&i.res.name);
                        if from == Some(Some(i.res.id)) {
                            option_el.ref_attr("selected", "");
                        }
                        out.ref_push(option_el);
                    }
                    return Ok(vec![out]);
                }
            })],
        }, Box::new(FormOptChannelGroupState {
            err_el: err_el,
            select_el: select_el,
        }))
    }
}

const RETENTION_KIND_NOTHING: &str = "nothing";
const RETENTION_KIND_VIEWED: &str = "viewed";
const RETENTION_KIND_ALL: &str = "all";
const RETENTION_KIND_DAYS: &str = "days";

struct FormRetentionState {
    err_el: El,
    select_el: El,
    days_el: El,
}

impl FormRetentionState {
    fn parse(&self) -> Result<ChannelRetention, ()> {
        self.err_el.ref_clear();
        let kind = self.select_el.raw().dyn_into::<HtmlSelectElement>().unwrap().value();
        match kind.as_str() {
            RETENTION_KIND_NOTHING => return Ok(ChannelRetention::Nothing),
            RETENTION_KIND_VIEWED => return Ok(ChannelRetention::Viewed),
            RETENTION_KIND_ALL => return Ok(ChannelRetention::All),
            RETENTION_KIND_DAYS => {
                let raw = self.days_el.raw().dyn_into::<HtmlInputElement>().unwrap().value();
                match raw.trim().parse::<u64>() {
                    Ok(days) if days > 0 => return Ok(ChannelRetention::Days(days)),
                    _ => {
                        self.err_el.ref_text("Enter a whole number of days greater than zero");
                        return Err(());
                    },
                }
            },
            other => {
                self.err_el.ref_text(&format!("Unknown retention kind [{}]", other));
                return Err(());
            },
        }
    }
}

fn new_form_retention(
    options: &[(&'static str, &'static str)],
    from: ChannelRetention,
) -> (rooting_forms::FormElements, FormRetentionState) {
    let err_el = err_el();
    let select_el = el("select");
    let days_el = el("input").attr("type", "number").attr("min", "1").attr("placeholder", "Days");
    let (current_kind, current_days) = match from {
        ChannelRetention::Nothing => (RETENTION_KIND_NOTHING, None),
        ChannelRetention::Viewed => (RETENTION_KIND_VIEWED, None),
        ChannelRetention::All => (RETENTION_KIND_ALL, None),
        ChannelRetention::Days(d) => (RETENTION_KIND_DAYS, Some(d)),
    };
    for (kind, label) in options {
        let option_el = el("option").attr("value", kind).text(label);
        if *kind == current_kind {
            option_el.ref_attr("selected", "");
        }
        select_el.ref_push(option_el);
    }
    if let Some(d) = current_days {
        days_el.ref_attr("value", &d.to_string());
    } else {
        days_el.ref_attr("hidden", "");
    }
    select_el.ref_on("change", {
        let select_el = select_el.weak();
        let days_el = days_el.weak();
        move |_| {
            let (Some(select_el), Some(days_el)) = (select_el.upgrade(), days_el.upgrade()) else {
                return;
            };
            let kind = select_el.raw().dyn_into::<HtmlSelectElement>().unwrap().value();
            if kind == RETENTION_KIND_DAYS {
                days_el.ref_remove_attr("hidden");
            } else {
                days_el.ref_attr("hidden", "");
            }
        }
    });
    return (rooting_forms::FormElements {
        error: Some(err_el.clone()),
        elements: vec![select_el.clone(), days_el.clone()],
    }, FormRetentionState {
        err_el: err_el,
        select_el: select_el,
        days_el: days_el,
    });
}

#[derive(Clone)]
pub struct FormMemberRetention(pub ChannelRetention);

impl rooting_forms::FormState<FormMemberRetention> for FormRetentionState {
    fn parse(&self) -> Result<FormMemberRetention, ()> {
        return Ok(FormMemberRetention(FormRetentionState::parse(self)?));
    }
}

impl<C: 'static + Clone> rooting_forms::FormWith<C> for FormMemberRetention {
    fn new_form_with_(
        _context: &C,
        _field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (rooting_forms::FormElements, Box<dyn rooting_forms::FormState<Self>>) {
        let (els, state) =
            new_form_retention(
                &[
                    (RETENTION_KIND_NOTHING, "Nothing, just browse"),
                    (RETENTION_KIND_VIEWED, "Only what's been viewed"),
                    (RETENTION_KIND_ALL, "Everything"),
                    (RETENTION_KIND_DAYS, "A number of days"),
                ],
                from.map(|x| x.0).unwrap_or(ChannelRetention::All),
            );
        return (els, Box::new(state));
    }
}

#[derive(Clone)]
pub struct FormOwnerRetention(pub ChannelRetention);

impl rooting_forms::FormState<FormOwnerRetention> for FormRetentionState {
    fn parse(&self) -> Result<FormOwnerRetention, ()> {
        return Ok(FormOwnerRetention(FormRetentionState::parse(self)?));
    }
}

impl<C: 'static + Clone> rooting_forms::FormWith<C> for FormOwnerRetention {
    fn new_form_with_(
        _context: &C,
        _field: &str,
        from: Option<&Self>,
        _depth: usize,
    ) -> (rooting_forms::FormElements, Box<dyn rooting_forms::FormState<Self>>) {
        let (els, state) =
            new_form_retention(
                &[(RETENTION_KIND_ALL, "Everything"), (RETENTION_KIND_DAYS, "A number of days")],
                from.map(|x| x.0).unwrap_or(ChannelRetention::All),
            );
        return (els, Box::new(state));
    }
}

pub fn build_form_inner(
    button_ok: &El,
    errs_el: El,
    form_els: Vec<El>,
    do_send: impl 'static + Clone + AsyncFn(f64) -> Result<(), String>,
) -> El {
    let idem = random();
    let errs_own_el = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    errs_el.ref_push(errs_own_el.clone());
    let form_el = el("form");
    form_el.ref_extend(form_els);
    button_ok.ref_on("click", {
        let thinking = Rc::new(RefCell::new(None));
        let errs_own_el = errs_own_el;
        let form_el = form_el.weak();
        move |_| {
            if thinking.borrow().is_some() {
                return;
            }
            errs_own_el.ref_clear();
            {
                let Some(form_el) = form_el.upgrade() else {
                    return;
                };
                form_el.ref_modify_classes(&[(&style_export::class_state_thinking().value, true)]);
            }
            *thinking.borrow_mut() = Some(scope_any(spawn_rooted({
                let form_el = form_el.clone();
                let thinking = thinking.clone();
                let errs_own_el = errs_own_el.clone();
                let do_send = do_send.clone();
                async move {
                    let res = do_send(idem).await;

                    // Unreachable if OK
                    let Some(form_el) = form_el.upgrade() else {
                        return;
                    };
                    *thinking.borrow_mut() = None;
                    form_el.ref_modify_classes(&[(&style_export::class_state_thinking().value, false)]);
                    if let Err(e) = res {
                        errs_own_el.ref_push(
                            style_export::leaf_err_block(style_export::LeafErrBlockArgs { data: e.to_string() }).root,
                        );
                    }
                }
            })));
        }
    });
    return form_el;
}

pub fn build_form(
    pc: &mut ProcessingContext,
    title: String,
    back_link: Ministate,
    form_err_el: Option<El>,
    form_els: Vec<El>,
    do_send: impl 'static + Clone + AsyncFn(f64) -> Result<(), String>,
) -> El {
    let head_bar = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&back_link),
        center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
            text: title,
            link: None,
        }).root,
        right: None,
    });
    head_bar
        .back_unread
        .ref_own(|el_| link!((_pc = pc), (unread = state().unread_any.clone()), (), (el_ = el_.weak()), {
            let el_ = el_.upgrade()?;
            el_.ref_modify_classes(&[(&style_export::class_state_hidden().value, !*unread.borrow())]);
        }));
    let form = style_export::cont_page_form(style_export::ContPageFormArgs {
        head_bar: head_bar.root,
        children: vec![],
    });
    form.errors.ref_extend(form_err_el.into_iter().collect());
    form.body.ref_push(build_form_inner(&form.submit, form.errors, form_els, do_send));
    return form.root;
}

pub fn build_nol_form<
    SEND: 'static + Clone + AsyncFn(f64) -> Result<(), String>,
>(
    pc: &mut ProcessingContext,
    back_link: &Ministate,
    title: &str,
    v: NowOrLater<(Option<El>, Vec<El>, SEND)>,
) -> El {
    let head_bar = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&back_link),
        center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
            text: title.to_string(),
            link: None,
        }).root,
        right: None,
    });
    head_bar
        .back_unread
        .ref_own(|el_| link!((_pc = pc), (unread = state().unread_any.clone()), (), (el_ = el_.weak()), {
            let el_ = el_.upgrade()?;
            el_.ref_modify_classes(&[(&style_export::class_state_hidden().value, !*unread.borrow())]);
        }));
    let form = style_export::cont_page_form(style_export::ContPageFormArgs {
        head_bar: head_bar.root,
        children: vec![],
    });
    let button_ok = form.submit;
    let errs = form.errors;
    let body;
    match v {
        NowOrLater::Now((form_err_el, form_els, do_send)) => {
            errs.ref_extend(form_err_el.into_iter().collect());
            body = vec![build_form_inner(&button_ok, errs, form_els, do_send)];
        },
        NowOrLater::Later(v) => {
            body = vec![el_async({
                let button_ok = button_ok.clone();
                async move {
                    let Some((form_err_el, form_els, do_send)) = v.get().await.map_err(|e| e.to_string())? else {
                        return Err(format!("Could not find object"));
                    };
                    errs.ref_extend(form_err_el.into_iter().collect());
                    return Ok(vec![build_form_inner(&button_ok, errs, form_els, do_send)]);
                }
            })];
        },
    }
    form.body.ref_extend(body);
    return form.root;
}

#[derive(Clone)]
pub struct EditPage<SEND> {
    pub center: El,
    pub submit_text: String,
    pub form_error: Option<El>,
    pub form: Vec<El>,
    pub menu: Vec<El>,
    pub send: SEND,
}

pub fn build_nol_edit_page<
    SEND: 'static + Clone + AsyncFn(f64) -> Result<(), String>,
>(pc: &mut ProcessingContext, back_link: &Ministate, v: NowOrLater<EditPage<SEND>>) -> El {
    let center = match &v {
        NowOrLater::Now(p) => p.center.clone(),
        NowOrLater::Later(_) => style_export::leaf_nonchat_head_bar_center_placeholder().root,
    };
    let head_bar = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&back_link),
        center: center.clone(),
        right: None,
    });
    head_bar
        .back_unread
        .ref_own(|el_| link!((_pc = pc), (unread = state().unread_any.clone()), (), (el_ = el_.weak()), {
            let el_ = el_.upgrade()?;
            el_.ref_modify_classes(&[(&style_export::class_state_hidden().value, !*unread.borrow())]);
        }));
    let form = style_export::cont_page_form(style_export::ContPageFormArgs {
        head_bar: head_bar.root,
        children: vec![],
    });
    let fill = {
        let submit = form.submit.clone();
        let errors = form.errors.clone();
        let menu = form.menu.clone();
        move |p: EditPage<SEND>| -> El {
            submit.ref_text(&p.submit_text);
            errors.ref_extend(p.form_error.into_iter().collect());
            menu.ref_extend(p.menu);
            return build_form_inner(&submit, errors, p.form, p.send);
        }
    };
    match v {
        NowOrLater::Now(p) => {
            form.body.ref_push(fill(p));
        },
        NowOrLater::Later(v) => {
            form.body.ref_push(el_async(async move {
                let Some(p) = v.get().await.map_err(|e| e.to_string())? else {
                    return Err(format!("Could not find object"));
                };
                center.ref_replace(vec![p.center.clone()]);
                return Ok(vec![fill(p)]);
            }));
        },
    }
    return form.root;
}

pub struct LazyPage {
    pub center: El,
    pub body: Vec<El>,
}

pub fn build_nol_menu<
    T: 'static + Clone,
>(
    pc: &mut ProcessingContext,
    back_link: &Ministate,
    v: NowOrLater<T>,
    build: impl 'static + FnOnce(T) -> LazyPage,
) -> El {
    let center;
    let body;
    match v {
        NowOrLater::Now(local) => {
            let r = build(local);
            center = r.center;
            body = r.body;
        },
        NowOrLater::Later(v) => {
            center = style_export::leaf_nonchat_head_bar_center_placeholder().root;
            body = vec![el_async({
                let center = center.clone();
                async move {
                    let Some(local) = v.get().await.map_err(|e| e.to_string())? else {
                        return Err(format!("Could not find object"));
                    };
                    let r = build(local);
                    center.ref_replace(vec![r.center]);
                    return Ok(r.body);
                }
            })];
        },
    }
    let head_bar = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&back_link),
        center: center,
        right: None,
    });
    head_bar
        .back_unread
        .ref_own(|el_| link!((_pc = pc), (unread = state().unread_any.clone()), (), (el_ = el_.weak()), {
            let el_ = el_.upgrade()?;
            el_.ref_modify_classes(&[(&style_export::class_state_hidden().value, !*unread.borrow())]);
        }));
    return style_export::cont_page_menu(style_export::ContPageMenuArgs {
        head_bar: head_bar.root,
        children: body,
    }).root;
}

pub fn build_nol_menu_title<
    T: 'static + Clone,
>(
    pc: &mut ProcessingContext,
    back_link: &Ministate,
    title: &str,
    v: NowOrLater<T>,
    build: impl 'static + FnOnce(T) -> El,
) -> El {
    let body;
    match v {
        NowOrLater::Now(local) => {
            body = build(local);
        },
        NowOrLater::Later(v) => {
            body = el_async({
                async move {
                    let Some(local) = v.get().await.map_err(|e| e.to_string())? else {
                        return Err(format!("Could not find object"));
                    };
                    return Ok(vec![build(local)]);
                }
            });
        },
    }
    let head_bar = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&back_link),
        center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
            text: title.to_string(),
            link: None,
        }).root,
        right: None,
    });
    head_bar
        .back_unread
        .ref_own(|el_| link!((_pc = pc), (unread = state().unread_any.clone()), (), (el_ = el_.weak()), {
            let el_ = el_.upgrade()?;
            el_.ref_modify_classes(&[(&style_export::class_state_hidden().value, !*unread.borrow())]);
        }));
    return style_export::cont_page_menu(style_export::ContPageMenuArgs {
        head_bar: head_bar.root,
        children: vec![body],
    }).root;
}

pub fn build_nol_chat_bar<
    T: 'static + Clone,
>(
    pc: &mut ProcessingContext,
    back_link: &Ministate,
    v: NowOrLater<T>,
    right: El,
    build: impl 'static + FnOnce(&mut ProcessingContext, T) -> El,
) -> style_export::ContChatHeadBarRet {
    let mut own = vec![];
    let center;
    match v {
        NowOrLater::Now(local) => {
            center = build(pc, local);
        },
        NowOrLater::Later(v) => {
            center = style_export::leaf_chat_head_bar_center_placeholder().root;
            own.push(spawn_rooted_log("Fetching channel info", {
                let top_center = center.clone();
                let eg = pc.eg();
                async move {
                    let Some(local) = v.get().await.map_err(|e| e.to_string())? else {
                        top_center.ref_replace(
                            vec![style_export::leaf_chat_head_bar_center(style_export::LeafChatHeadBarCenterArgs {
                                text: format!("Unknown"),
                                link: None,
                            }).root],
                        );
                        return Ok(());
                    };
                    let built = eg.event(|pc| build(pc, local)).unwrap();
                    top_center.ref_replace(vec![built]);
                    return Ok(());
                }
            }));
        },
    }
    let out = style_export::cont_chat_head_bar(style_export::ContChatHeadBarArgs {
        back_link: ministate_octothorpe(&back_link),
        center: center,
        right: Some(right),
    });
    out.root.ref_own(move |_| own);
    return out;
}

pub fn build_url_menu_items(url: &KwaUrl) -> Vec<El> {
    let url = url.to_string();
    let code = style_export::leaf_menu_code(style_export::LeafMenuCodeArgs { text: url.clone() });
    code.copy.ref_on("click", {
        let url = url.clone();
        move |_| copy_text(&state().log, &url)
    });
    return vec![
        //. .
        style_export::leaf_menu_qr(
            style_export::LeafMenuQrArgs {
                svg: QrCode::new(url.as_bytes())
                    .unwrap()
                    .render::<svg::Color>()
                    .light_color(svg::Color("transparent"))
                    .build(),
            },
        ).root,
        code.root,
    ];
}
