use {
    crate::{
        js::{
            js_err,
            style_export,
            StopStream,
        },
        page_channel_join_url::FormKwaUrlInvite,
        state::{
            ChannelJoinSource,
            Ministate,
            MinistateChannelJoinConfirm,
            goto_replace_ministate,
            ministate_octothorpe,
            state,
        },
    },
    gloo::{
        timers::future::TimeoutFuture,
        utils::{
            document,
            window,
        },
    },
    js_sys::{
        Object,
        Reflect,
    },
    lunk::{
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        spawn_rooted,
    },
    std::str::FromStr,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        CanvasRenderingContext2d,
        HtmlCanvasElement,
        HtmlMediaElement,
        HtmlVideoElement,
        MediaStream,
        MediaStreamConstraints,
    },
};

pub fn build(pc: &mut ProcessingContext) -> El {
    let eg = pc.eg();
    let head_bar = style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
        back_link: ministate_octothorpe(&Ministate::TopAdd),
        center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
            text: format!("Join by QR"),
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
    let scanner = style_export::leaf_menu_qr_scanner();
    let video = scanner.video.raw().dyn_into::<HtmlVideoElement>().unwrap();
    let status = scanner.status.clone();
    let root = style_export::cont_page_menu(style_export::ContPageMenuArgs {
        head_bar: head_bar.root,
        children: vec![scanner.root],
    }).root;
    root.ref_own(|_| spawn_rooted(async move {
        let video_constraints = Object::new();
        let facing_mode = Object::new();
        Reflect::set(&facing_mode, &JsValue::from_str("ideal"), &JsValue::from_str("environment")).unwrap();
        Reflect::set(&video_constraints, &JsValue::from_str("facingMode"), &facing_mode).unwrap();
        let constraints = MediaStreamConstraints::new();
        constraints.set_audio(&JsValue::FALSE);
        constraints.set_video(&video_constraints);
        let media_devices = match window().navigator().media_devices() {
            Ok(m) => m,
            Err(e) => {
                status.ref_text(&js_err("Camera access is unavailable", e));
                return;
            },
        };
        let stream_promise = match media_devices.get_user_media_with_constraints(&constraints) {
            Ok(p) => p,
            Err(e) => {
                status.ref_text(&js_err("Error requesting camera", e));
                return;
            },
        };
        let stream = match JsFuture::from(stream_promise).await {
            Ok(s) => s.dyn_into::<MediaStream>().unwrap(),
            Err(e) => {
                status.ref_text(&js_err("Error opening camera", e));
                return;
            },
        };
        let _stop_stream = StopStream(stream.clone());
        video.set_src_object(Some(&stream));
        let canvas = document().create_element("canvas").unwrap().dyn_into::<HtmlCanvasElement>().unwrap();
        let ctx_options = Object::new();
        Reflect::set(&ctx_options, &JsValue::from_str("willReadFrequently"), &JsValue::TRUE).unwrap();
        let ctx =
            canvas
                .get_context_with_context_options("2d", &ctx_options)
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap();
        loop {
            TimeoutFuture::new(150).await;
            if video.ready_state() < HtmlMediaElement::HAVE_CURRENT_DATA {
                continue;
            }
            let source_w = video.video_width();
            let source_h = video.video_height();
            if source_w == 0 || source_h == 0 {
                continue;
            }
            let scale = (640 as f64 / source_w.max(source_h) as f64).min(1.);
            let w = (source_w as f64 * scale).round() as u32;
            let h = (source_h as f64 * scale).round() as u32;
            if canvas.width() != w {
                canvas.set_width(w);
            }
            if canvas.height() != h {
                canvas.set_height(h);
            }
            if let Err(e) = ctx.draw_image_with_html_video_element_and_dw_and_dh(&video, 0., 0., w as f64, h as f64) {
                status.ref_text(&js_err("Error drawing camera frame", e));
                continue;
            }
            let rgba = match ctx.get_image_data(0, 0, w as i32, h as i32) {
                Ok(d) => d.data().0,
                Err(e) => {
                    status.ref_text(&js_err("Error reading camera frame", e));
                    continue;
                },
            };
            let w = w as usize;
            let h = h as usize;
            let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| {
                let i = (y * w + x) * 4;
                return ((rgba[i] as u32 * 299 + rgba[i + 1] as u32 * 587 + rgba[i + 2] as u32 * 114) / 1000) as u8;
            });
            let mut texts = vec![];
            for grid in prepared.detect_grids() {
                if let Ok((_, text)) = grid.decode() {
                    texts.push(text);
                }
            }
            for text in texts {
                match FormKwaUrlInvite::from_str(&text) {
                    Ok(invite) => {
                        eg.event(|pc| {
                            goto_replace_ministate(
                                pc,
                                &state().log,
                                &Ministate::ChannelJoinConfirm(MinistateChannelJoinConfirm {
                                    url: invite.into_url(),
                                    source: ChannelJoinSource::Qr,
                                }),
                            );
                        }).unwrap();
                        return;
                    },
                    Err(e) => {
                        status.ref_text(&format!("Scanned QR code isn't a valid invitation: {}", e));
                    },
                }
            }
        }
    }));
    return root;
}
