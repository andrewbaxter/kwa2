use {
    crate::{
        blocks::ViewFile,
        js::style_export,
        modal::{
            close_modal,
            show_modal,
        },
    },
    gloo::{
        events::EventListener,
        timers::callback::Timeout,
        utils::window,
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        rc::Rc,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        KeyboardEvent,
        TouchEvent,
        WheelEvent,
    },
};

pub fn open_media_viewer(files: Vec<ViewFile>, index: usize) {
    let viewer = style_export::cont_chat_modal_media_viewer();
    let files = Rc::new(files);
    let current = Rc::new(Cell::new(index));
    let current_is_video = Rc::new(Cell::new(false));
    let show: Rc<dyn Fn(usize)> = Rc::new({
        let files = files.clone();
        let current = current.clone();
        let current_is_video = current_is_video.clone();
        let stage = viewer.stage.clone();
        let counter = viewer.counter.clone();
        let download = viewer.download.clone();
        let prev = viewer.prev.clone();
        let next = viewer.next.clone();
        move |i| {
            current.set(i);
            let file = &files[i];
            stage.ref_clear();
            current_is_video.set(file.is_video());
            if file.is_video() {
                stage.ref_push(
                    style_export::leaf_chat_modal_media_viewer_video(
                        style_export::LeafChatModalMediaViewerVideoArgs { src: file.url(false) },
                    ).root,
                );
            } else {
                stage.ref_push(
                    style_export::leaf_chat_modal_media_viewer_image(
                        style_export::LeafChatModalMediaViewerImageArgs { src: file.url(false) },
                    ).root,
                );
            }
            download.ref_attr("href", &file.url(false));
            download.ref_attr("download", &file.download_name());
            counter.ref_text(&format!("{} / {}", i + 1, files.len()));
            prev.ref_modify_classes(&[(&style_export::class_state_hidden().value, i == 0)]);
            next.ref_modify_classes(&[(&style_export::class_state_hidden().value, i + 1 >= files.len())]);
        }
    });
    let step: Rc<dyn Fn(isize)> = Rc::new({
        let files = files.clone();
        let current = current.clone();
        let show = show.clone();
        move |delta| {
            let target = current.get() as isize + delta;
            if target < 0 || target as usize >= files.len() {
                return;
            }
            show(target as usize);
        }
    });
    let hide_timer = Rc::new(RefCell::new(None::<Timeout>));
    let poke: Rc<dyn Fn()> = Rc::new({
        let root = viewer.root.weak();
        let hide_timer = hide_timer.clone();
        move || {
            let Some(root) = root.upgrade() else {
                return;
            };
            root.ref_modify_classes(&[(&style_export::class_state_controls_hidden().value, false)]);
            *hide_timer.borrow_mut() = Some(Timeout::new(2500, {
                let root = root.weak();
                move || {
                    let Some(root) = root.upgrade() else {
                        return;
                    };
                    root.ref_modify_classes(&[(&style_export::class_state_controls_hidden().value, true)]);
                }
            }));
        }
    });
    show(index);
    poke();
    viewer.download.ref_on("click", |ev| ev.stop_propagation());
    viewer.close.ref_on("click", |ev| {
        ev.stop_propagation();
        close_modal();
    });
    viewer.prev.ref_on("click", {
        let step = step.clone();
        move |ev| {
            ev.stop_propagation();
            step(-1);
        }
    });
    viewer.next.ref_on("click", {
        let step = step.clone();
        move |ev| {
            ev.stop_propagation();
            step(1);
        }
    });
    viewer.root.ref_on("mousemove", {
        let poke = poke.clone();
        move |_| poke()
    });
    viewer.root.ref_on("click", {
        let poke = poke.clone();
        move |_| poke()
    });
    let last_wheel = Rc::new(Cell::new(None::<f64>));
    viewer.root.ref_on("wheel", {
        let step = step.clone();
        let last_wheel = last_wheel.clone();
        move |ev| {
            let ev = ev.dyn_ref::<WheelEvent>().unwrap();
            let delta = if ev.delta_y().abs() >= ev.delta_x().abs() {
                ev.delta_y()
            } else {
                ev.delta_x()
            };
            if delta.abs() < 30. {
                return;
            }
            let now = js_sys::Date::now();
            if let Some(last) = last_wheel.get() {
                if now - last < 300. {
                    return;
                }
            }
            last_wheel.set(Some(now));
            step(if delta > 0. {
                1
            } else {
                -1
            });
        }
    });
    let touch_start = Rc::new(Cell::new(None::<(f64, f64)>));
    viewer.root.ref_on("touchstart", {
        let touch_start = touch_start.clone();
        let current_is_video = current_is_video.clone();
        let poke = poke.clone();
        move |ev| {
            poke();
            let ev = ev.dyn_ref::<TouchEvent>().unwrap();
            let Some(touch) = ev.touches().get(0) else {
                return;
            };
            let x = touch.client_x() as f64;
            let y = touch.client_y() as f64;
            let height = window().inner_height().unwrap().as_f64().unwrap();
            if current_is_video.get() && y > height * 0.8 {
                touch_start.set(None);
                return;
            }
            touch_start.set(Some((x, y)));
        }
    });
    viewer.root.ref_on("touchend", {
        let touch_start = touch_start.clone();
        let step = step.clone();
        move |ev| {
            let ev = ev.dyn_ref::<TouchEvent>().unwrap();
            let Some((start_x, start_y)) = touch_start.take() else {
                return;
            };
            let Some(touch) = ev.changed_touches().get(0) else {
                return;
            };
            let dx = touch.client_x() as f64 - start_x;
            let dy = touch.client_y() as f64 - start_y;
            if dx.abs() < 50. || dx.abs() < dy.abs() {
                return;
            }
            step(if dx < 0. {
                1
            } else {
                -1
            });
        }
    });
    viewer.root.ref_own(|_| EventListener::new(&window(), "keydown", {
        let step = step.clone();
        move |ev| {
            let ev = ev.dyn_ref::<KeyboardEvent>().unwrap();
            match ev.key().as_str() {
                "ArrowLeft" => step(-1),
                "ArrowRight" => step(1),
                "Escape" => close_modal(),
                _ => { },
            }
        }
    }));
    show_modal(viewer.root);
}
