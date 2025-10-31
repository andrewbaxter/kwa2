use {
    crate::libmain::{
        localdata::{
            req_api_channelgroups,
            req_api_identities,
        },
        state::{
            ministate_octothorpe,
            Ministate,
        },
    },
    flowcontrol::ta_return,
    js_sys::Math::random,
    rooting::{
        el,
        scope_any,
        spawn_rooted,
        El,
    },
    rooting_forms::css::err_el,
    shared::interface::wire::shared::ChannelGroupId,
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm::js::{
        el_async,
        style_export,
    },
    wasm_bindgen::JsCast,
    web_sys::HtmlSelectElement,
};

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
                        out.ref_push(el("option").attr("value", &value).text(&i.res.memo_short));
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
                        out.ref_push(el("option").attr("value", &value).text(&i.res.memo_short));
                        if let Some(s) = &from {
                            if *s == Some(i.res.id) {
                                out.ref_attr("value", &value);
                            }
                        }
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

pub fn build_form(
    title: String,
    back_link: Ministate,
    form_err_el: El,
    form_els: Vec<El>,
    do_send: impl 'static + Clone + AsyncFn(f64) -> Result<(), String>,
) -> El {
    let idem = random();
    let button_ok = style_export::leaf_page_form_button_submit().root;
    let errs_el = style_export::cont_page_form_errors().root;
    let errs_own_el = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    errs_el.ref_push(errs_own_el.clone());
    errs_el.ref_push(form_err_el);
    let form_el = el("form");
    form_el.ref_push(errs_el.clone());
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
    return style_export::cont_page_form(style_export::ContPageFormArgs {
        edit_bar_children: vec![button_ok],
        children: vec![
            //. .
            style_export::cont_menu_bar(style_export::ContMenuBarArgs {
                back_link: ministate_octothorpe(&back_link),
                text: title,
                center_link: None,
                right: None,
            }).root,
            form_el,
        ],
    }).root;
}
