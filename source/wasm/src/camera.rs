use {
    crate::{
        blocks::{
            format_size,
            LocalBlob,
        },
        js::{
            js_err,
            style_export,
            StopStream,
        },
        modal::ModalHandle,
    },
    futures::channel::oneshot::{
        self,
        Receiver,
    },
    gloo::utils::{
        document,
        window,
    },
    js_sys::{
        Array,
        Object,
        Reflect,
    },
    rooting::{
        el,
        spawn_rooted,
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        rc::Rc,
    },
    wasm_bindgen::{
        closure::Closure,
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::{
        spawn_local,
        JsFuture,
    },
    web_sys::{
        Blob,
        BlobEvent,
        BlobPropertyBag,
        CanvasRenderingContext2d,
        HtmlCanvasElement,
        HtmlImageElement,
        HtmlSelectElement,
        HtmlVideoElement,
        MediaRecorder,
        MediaStream,
        MediaStreamConstraints,
    },
};

#[derive(Clone, Copy, PartialEq)]
enum CameraMode {
    Photo,
    Video,
}

struct Recording {
    recorder: MediaRecorder,
    _on_data: Closure<dyn FnMut(BlobEvent)>,
    _on_stop: Closure<dyn FnMut(JsValue)>,
}

struct CameraState {
    mode: Cell<CameraMode>,
    facing_user: Cell<bool>,
    stream: RefCell<Option<StopStream>>,
    opening: RefCell<Option<Receiver<()>>>,
    recording: RefCell<Option<Recording>>,
}

async fn canvas_blob(canvas: &HtmlCanvasElement, quality: f64) -> Result<Blob, String> {
    let (tx, rx) = oneshot::channel::<JsValue>();
    let cb = Closure::once(move |b: JsValue| {
        _ = tx.send(b);
    });
    canvas
        .to_blob_with_type_and_encoder_options(cb.as_ref().unchecked_ref(), "image/jpeg", &JsValue::from_f64(quality))
        .map_err(|e| js_err("Error encoding photo", e))?;
    let blob = rx.await.map_err(|_| format!("Photo encoding was cancelled"))?;
    drop(cb);
    if blob.is_null() {
        return Err(format!("Photo encoding produced no image"));
    }
    return Ok(blob.dyn_into::<Blob>().unwrap());
}

async fn scaled_blob(source: &HtmlCanvasElement, max_edge: u32, quality: f64) -> Result<Blob, String> {
    let w = source.width();
    let h = source.height();
    let scale = (max_edge as f64 / w.max(h) as f64).min(1.);
    let canvas = document().create_element("canvas").unwrap().dyn_into::<HtmlCanvasElement>().unwrap();
    canvas.set_width((w as f64 * scale).round() as u32);
    canvas.set_height((h as f64 * scale).round() as u32);
    let ctx = canvas.get_context("2d").unwrap().unwrap().dyn_into::<CanvasRenderingContext2d>().unwrap();
    ctx
        .draw_image_with_html_canvas_element_and_dw_and_dh(
            source,
            0.,
            0.,
            canvas.width() as f64,
            canvas.height() as f64,
        )
        .map_err(|e| js_err("Error scaling photo", e))?;
    return canvas_blob(&canvas, quality).await;
}

pub fn open_camera(on_accept: Rc<dyn Fn(Blob)>) {
    let cam = style_export::cont_chat_editor_modal_camera();
    let cam_handle = ModalHandle::new(&cam.root);
    let video = cam.video.raw().dyn_into::<HtmlVideoElement>().unwrap();
    let state = Rc::new(CameraState {
        mode: Cell::new(CameraMode::Photo),
        facing_user: Cell::new(false),
        stream: RefCell::new(None),
        opening: RefCell::new(None),
        recording: RefCell::new(None),
    });
    let refresh_buttons: Rc<dyn Fn()> = Rc::new({
        let state = Rc::downgrade(&state);
        let mode_photo = cam.mode_photo.weak();
        let mode_video = cam.mode_video.weak();
        let shutter = cam.shutter.weak();
        move || {
            let Some(state) = state.upgrade() else {
                return;
            };
            let selected = &style_export::class_state_selected().value;
            if let Some(e) = mode_photo.upgrade() {
                e.ref_modify_classes(&[(selected, state.mode.get() == CameraMode::Photo)]);
            }
            if let Some(e) = mode_video.upgrade() {
                e.ref_modify_classes(&[(selected, state.mode.get() == CameraMode::Video)]);
            }
            if let Some(e) = shutter.upgrade() {
                e.ref_modify_classes(
                    &[
                        (&style_export::class_state_recording().value, state.recording.borrow().is_some()),
                        (&style_export::class_state_video().value, state.mode.get() == CameraMode::Video),
                    ],
                );
            }
        }
    });
    let open_stream: Rc<dyn Fn()> = Rc::new({
        let state = Rc::downgrade(&state);
        let video = video.clone();
        let status = cam.status.weak();
        move || {
            let Some(state1) = state.upgrade() else {
                return;
            };
            *state1.stream.borrow_mut() = None;
            let video = video.clone();
            let status = status.clone();
            let state = state.clone();
            *state1.opening.borrow_mut() = Some(spawn_rooted(async move {
                let set_status = |text: &str| {
                    if let Some(status) = status.upgrade() {
                        status.ref_text(text);
                    }
                };
                let Some(state) = state.upgrade() else {
                    return;
                };
                set_status("");
                let video_constraints = Object::new();
                let facing_mode = Object::new();
                Reflect::set(&facing_mode, &JsValue::from_str("ideal"), &JsValue::from_str(if state.facing_user.get() {
                    "user"
                } else {
                    "environment"
                })).unwrap();
                Reflect::set(&video_constraints, &JsValue::from_str("facingMode"), &facing_mode).unwrap();
                let constraints = MediaStreamConstraints::new();
                constraints.set_audio(&JsValue::from_bool(state.mode.get() == CameraMode::Video));
                constraints.set_video(&video_constraints);
                let media_devices = match window().navigator().media_devices() {
                    Ok(m) => m,
                    Err(e) => {
                        set_status(&js_err("Camera access is unavailable", e));
                        return;
                    },
                };
                let stream_promise = match media_devices.get_user_media_with_constraints(&constraints) {
                    Ok(p) => p,
                    Err(e) => {
                        set_status(&js_err("Error requesting camera", e));
                        return;
                    },
                };
                let stream = match JsFuture::from(stream_promise).await {
                    Ok(s) => s.dyn_into::<MediaStream>().unwrap(),
                    Err(e) => {
                        set_status(&js_err("Error opening camera", e));
                        return;
                    },
                };
                video.set_src_object(Some(&stream));
                *state.stream.borrow_mut() = Some(StopStream(stream));
            }));
        }
    });
    cam.close.ref_on("click", {
        let cam_handle = cam_handle.clone();
        move |_| cam_handle.close()
    });
    cam.flip.ref_on("click", {
        let state = Rc::downgrade(&state);
        let open_stream = open_stream.clone();
        move |_| {
            let Some(state) = state.upgrade() else {
                return;
            };
            if state.recording.borrow().is_some() {
                return;
            }
            state.facing_user.set(!state.facing_user.get());
            open_stream();
        }
    });
    for (button, mode) in [(&cam.mode_photo, CameraMode::Photo), (&cam.mode_video, CameraMode::Video)] {
        button.ref_on("click", {
            let state = Rc::downgrade(&state);
            let open_stream = open_stream.clone();
            let refresh_buttons = refresh_buttons.clone();
            move |_| {
                let Some(state) = state.upgrade() else {
                    return;
                };
                if state.recording.borrow().is_some() || state.mode.get() == mode {
                    return;
                }
                state.mode.set(mode);
                refresh_buttons();
                open_stream();
            }
        });
    }
    cam.shutter.ref_on("click", {
        let cam_handle = cam_handle.clone();
        let state = Rc::downgrade(&state);
        let video = video.clone();
        let status = cam.status.weak();
        let root = cam.root.weak();
        let refresh_buttons = refresh_buttons.clone();
        move |_| {
            let Some(state1) = state.upgrade() else {
                return;
            };
            let Some(root) = root.upgrade() else {
                return;
            };
            let set_status = {
                let status = status.clone();
                move |text: &str| {
                    if let Some(status) = status.upgrade() {
                        status.ref_text(text);
                    }
                }
            };
            match state1.mode.get() {
                CameraMode::Photo => {
                    let w = video.video_width();
                    let h = video.video_height();
                    if w == 0 || h == 0 {
                        return;
                    }
                    let canvas =
                        document().create_element("canvas").unwrap().dyn_into::<HtmlCanvasElement>().unwrap();
                    canvas.set_width(w);
                    canvas.set_height(h);
                    let ctx =
                        canvas.get_context("2d").unwrap().unwrap().dyn_into::<CanvasRenderingContext2d>().unwrap();
                    if let Err(e) = ctx.draw_image_with_html_video_element(&video, 0., 0.) {
                        set_status(&js_err("Error capturing photo", e));
                        return;
                    }
                    let on_accept = on_accept.clone();
                    let cam_handle = cam_handle.clone();
                    root.ref_own(|_| spawn_rooted(async move {
                        let res = async {
                            let full = canvas_blob(&canvas, 0.92).await?;
                            let high = scaled_blob(&canvas, 2560, 0.9).await?;
                            let low = scaled_blob(&canvas, 1280, 0.75).await?;
                            return Ok::<_, String>(
                                vec![
                                    (format!("Full size"), full),
                                    (format!("High quality"), high),
                                    (format!("Low quality"), low),
                                ],
                            );
                        }.await;
                        let options = match res {
                            Ok(options) => options,
                            Err(e) => {
                                set_status(&e);
                                return;
                            },
                        };
                        let options =
                            options
                                .into_iter()
                                .map(|(label, blob)| (label, LocalBlob::new(blob)))
                                .collect::<Vec<_>>();
                        let image =
                            style_export::leaf_chat_editor_modal_camera_preview_image(
                                style_export::LeafChatEditorModalCameraPreviewImageArgs {
                                    src: options[0].1.url.clone(),
                                },
                            ).root;
                        let mut option_els = vec![];
                        for (i, (label, blob)) in options.iter().enumerate() {
                            option_els.push(
                                el("option")
                                    .attr("value", &i.to_string())
                                    .text(&format!("{} - {}", label, format_size(blob.blob.size() as u64))),
                            );
                        }
                        let select =
                            style_export::leaf_chat_editor_modal_camera_preview_select(
                                style_export::LeafChatEditorModalCameraPreviewSelectArgs { children: option_els },
                            ).root;
                        let options = Rc::new(options);
                        select.ref_on("change", {
                            let options = options.clone();
                            let image = image.weak();
                            let select = select.weak();
                            move |_| {
                                let Some(image) = image.upgrade() else {
                                    return;
                                };
                                let Some(select) = select.upgrade() else {
                                    return;
                                };
                                let i =
                                    select.raw().dyn_into::<HtmlSelectElement>().unwrap().selected_index() as usize;
                                image.raw().dyn_into::<HtmlImageElement>().unwrap().set_src(&options[i].1.url);
                            }
                        });
                        let preview =
                            style_export::cont_chat_editor_modal_camera_preview(
                                style_export::ContChatEditorModalCameraPreviewArgs {
                                    media: image,
                                    options: Some(select.clone()),
                                },
                            );
                        let preview_handle = ModalHandle::new(&preview.root);
                        preview.retake.ref_on("click", {
                            let on_accept = on_accept.clone();
                            let preview_handle = preview_handle.clone();
                            move |_| {
                                preview_handle.close();
                                open_camera(on_accept.clone());
                            }
                        });
                        preview.accept.ref_on("click", {
                            let preview_handle = preview_handle.clone();
                            let options = options.clone();
                            let select = select.weak();
                            move |_| {
                                let Some(select) = select.upgrade() else {
                                    return;
                                };
                                let i =
                                    select.raw().dyn_into::<HtmlSelectElement>().unwrap().selected_index() as usize;
                                let blob = options[i].1.blob.clone();
                                preview_handle.close();
                                on_accept(blob);
                            }
                        });
                        preview.root.ref_own(|_| options);
                        cam_handle.close();
                        preview_handle.open();
                    }));
                },
                CameraMode::Video => {
                    if let Some(recording) = &*state1.recording.borrow() {
                        if let Err(e) = recording.recorder.stop() {
                            set_status(&js_err("Error stopping recording", e));
                        }
                        return;
                    }
                    let Some(stream) = state1.stream.borrow().as_ref().map(|s| s.0.clone()) else {
                        return;
                    };
                    let recorder = match MediaRecorder::new_with_media_stream(&stream) {
                        Ok(r) => r,
                        Err(e) => {
                            set_status(&js_err("Error starting recording", e));
                            return;
                        },
                    };
                    let chunks = Rc::new(RefCell::new(vec![]));
                    let on_data = Closure::<dyn FnMut(BlobEvent)>::new({
                        let chunks = chunks.clone();
                        move |ev: BlobEvent| {
                            if let Some(data) = ev.data() {
                                chunks.borrow_mut().push(data);
                            }
                        }
                    });
                    let on_stop = Closure::<dyn FnMut(JsValue)>::new({
                        let state = state.clone();
                        let cam_handle = cam_handle.clone();
                        let chunks = chunks.clone();
                        let on_accept = on_accept.clone();
                        let set_status = set_status.clone();
                        move |_: JsValue| {
                            let Some(state) = state.upgrade() else {
                                return;
                            };
                            let Some(recording) = state.recording.borrow_mut().take() else {
                                return;
                            };
                            let parts = Array::new();
                            for chunk in chunks.borrow().iter() {
                                parts.push(chunk);
                            }
                            let props = BlobPropertyBag::new();
                            props.set_type(&recording.recorder.mime_type());
                            spawn_local(async move {
                                drop(recording);
                            });
                            let blob = match Blob::new_with_blob_sequence_and_options(&parts, &props) {
                                Ok(blob) => blob,
                                Err(e) => {
                                    set_status(&js_err("Error assembling recording", e));
                                    return;
                                },
                            };
                            let on_accept = on_accept.clone();
                            let local = LocalBlob::new(blob);
                            let preview =
                                style_export::cont_chat_editor_modal_camera_preview(
                                    style_export::ContChatEditorModalCameraPreviewArgs {
                                        media: style_export::leaf_chat_editor_modal_camera_preview_video(
                                            style_export::LeafChatEditorModalCameraPreviewVideoArgs {
                                                src: local.url.clone(),
                                            },
                                        ).root,
                                        options: None,
                                    },
                                );
                            let preview_handle = ModalHandle::new(&preview.root);
                            preview.retake.ref_on("click", {
                                let on_accept = on_accept.clone();
                                let preview_handle = preview_handle.clone();
                                move |_| {
                                    preview_handle.close();
                                    open_camera(on_accept.clone());
                                }
                            });
                            preview.accept.ref_on("click", {
                                let preview_handle = preview_handle.clone();
                                let local = local.clone();
                                move |_| {
                                    let blob = local.blob.clone();
                                    preview_handle.close();
                                    on_accept(blob);
                                }
                            });
                            preview.root.ref_own(|_| local);
                            cam_handle.close();
                            preview_handle.open();
                        }
                    });
                    recorder.set_ondataavailable(Some(on_data.as_ref().unchecked_ref()));
                    recorder.set_onstop(Some(on_stop.as_ref().unchecked_ref()));
                    if let Err(e) = recorder.start() {
                        set_status(&js_err("Error starting recording", e));
                        return;
                    }
                    *state1.recording.borrow_mut() = Some(Recording {
                        recorder: recorder,
                        _on_data: on_data,
                        _on_stop: on_stop,
                    });
                    refresh_buttons();
                },
            }
        }
    });
    refresh_buttons();
    open_stream();
    cam.root.ref_own(|_| (state, open_stream, refresh_buttons));
    cam_handle.open();
}
