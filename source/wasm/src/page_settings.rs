use {
    crate::{
        calendar::LOCALSTORAGE_WEEK_START,
        api::{
            req_get,
            req_post_json,
        },
        js::{
            Engine,
            configure_async_button_once,
            el_async,
            style_export,
        },
        state::{
            Ministate,
            ministate_octothorpe,
            state,
        },
    },
    flowcontrol::{
        superif,
        ta_return,
    },
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    js_sys::JSON,
    shared::{
        calendar::{
            weekday_from_jiff,
            weekday_to_jiff,
        },
        interface::shared::EventWeekday,
    },
    rooting::El,
    shared::interface::wire::c2s,
    wasm_bindgen::JsValue,
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        Notification,
        NotificationPermission,
        PushManager,
        PushSubscription,
        PushSubscriptionOptionsInit,
    },
};

pub fn build() -> El {
    return style_export::cont_page_menu(style_export::ContPageMenuArgs {
        head_bar: style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                text: format!("Settings"),
                link: None,
            }).root,
            right: None,
        }).root,
        children: vec![
            //. .
            el_async(async move {
                ta_return!(Vec < El >, String);
                if state().env.engine == Some(Engine::IosSafari) && !state().env.pwa {
                    return Ok(
                        vec![
                            style_export::leaf_err_block(
                                style_export::LeafErrBlockArgs {
                                    data: format!("This app must be added to the home screen to enable notifications"),
                                },
                            ).root,
                        ],
                    );
                }
                let sw = state().service_worker.get().await?;
                let pm = sw.push_manager().map_err(|e| format!("{:?}", e.as_string()))?;

                async fn build_notification_button(pm: PushManager) -> El {
                    match async {
                        let button;
                        superif!({
                            let sub =
                                JsFuture::from(
                                    pm
                                        .get_subscription()
                                        .map_err(|e| format!("Error getting subscription: {:?}", e.as_string()))?,
                                )
                                    .await
                                    .map_err(|e| format!("Error getting subscription (async): {:?}", e.as_string()))?;
                            if sub.is_null() {
                                break 'no;
                            }
                            match Notification::permission() {
                                NotificationPermission::Default => {
                                    break 'no;
                                },
                                NotificationPermission::Denied => {
                                    break 'no;
                                },
                                NotificationPermission::Granted => { },
                                _ => break 'no,
                            }
                            button =
                                style_export::leaf_menu_button(
                                    style_export::LeafMenuButtonArgs { text: "Re-enable notifications".to_string() },
                                ).root;
                        } 'no {
                            button =
                                style_export::leaf_menu_button(
                                    style_export::LeafMenuButtonArgs { text: "Enable notifications".to_string() },
                                ).root;
                        });
                        configure_async_button_once(&button, {
                            let button = button.weak();
                            async move || {
                                let replacement = match async {
                                    match NotificationPermission::from_js_value(
                                        &JsFuture::from(
                                            Notification
                                            ::request_permission().map_err(
                                                |e| format!(
                                                    "Error requesting permissions for notifications: {:?}",
                                                    e.as_string()
                                                ),
                                            )?,
                                        )
                                            .await
                                            .map_err(
                                                |e| format!(
                                                    "Error requesting permissions for notifications: {:?}",
                                                    e.as_string()
                                                ),
                                            )?,
                                    ).expect("Notification permission result didn't match permission struct") {
                                        NotificationPermission::Granted => { },
                                        _ => return Err(format!("Notification permission denied")),
                                    }
                                    let key = req_get(c2s::NotificationServerKey).await?;
                                    let sub =
                                        PushSubscription::from(
                                            JsFuture::from(
                                                pm
                                                    .subscribe_with_options(&{
                                                        let o = PushSubscriptionOptionsInit::new();
                                                        o.set_user_visible_only(true);
                                                        o.set_application_server_key(&JsValue::from(key));
                                                        o
                                                    })
                                                    .map_err(
                                                        |e| format!(
                                                            "Error setting up subscription for notifications: {:?}",
                                                            e.as_string()
                                                        ),
                                                    )?,
                                            )
                                                .await
                                                .map_err(
                                                    |e| format!(
                                                        "Error setting up subscription for notifications (async): {:?}",
                                                        e.as_string()
                                                    ),
                                                )?,
                                        );
                                    req_post_json(
                                        &state().env.base_url,
                                        c2s::NotificationRegister {
                                            data: serde_json::from_str(
                                                &JSON::stringify(
                                                    &sub
                                                        .to_json()
                                                        .map_err(
                                                            |e| format!(
                                                                "Error converting subscription result into JSON to tell server: {:?}",
                                                                e.as_string()
                                                            ),
                                                        )?
                                                        .into(),
                                                )
                                                    .unwrap()
                                                    .as_string()
                                                    .unwrap(),
                                            ).unwrap(),
                                        },
                                    ).await?;
                                    return Ok(build_notification_button(pm).await);
                                }.await {
                                    Ok(e) => e,
                                    Err(e) => style_export::leaf_err_block(
                                        style_export::LeafErrBlockArgs { data: e },
                                    ).root,
                                };
                                let Some(button) = button.upgrade() else {
                                    return;
                                };
                                button.ref_replace(vec![replacement]);
                            }
                        });
                        return Ok(button);
                    }.await {
                        Ok(e) => return e,
                        Err(e) => return style_export::leaf_err_block(
                            style_export::LeafErrBlockArgs { data: e },
                        ).root,
                    }
                }

                return Ok(vec![build_notification_button(pm).await]);
            }),
            {
                let week_start_text = || -> String {
                    let name = |w: jiff::civil::Weekday| -> String {
                        return crate::js::intl_weekday(w, "long");
                    };
                    match LocalStorage::get::<EventWeekday>(LOCALSTORAGE_WEEK_START) {
                        Ok(w) => return format!("Weeks start on {}", name(weekday_to_jiff(w))),
                        Err(_) => return format!("Weeks start on {} (locale)", name(crate::js::locale_week_start())),
                    }
                };
                let button =
                    style_export::leaf_menu_button(
                        style_export::LeafMenuButtonArgs { text: week_start_text() },
                    ).root;
                button.ref_on("click", {
                    let button = button.weak();
                    move |_| {
                        let Some(button) = button.upgrade() else {
                            return;
                        };
                        let next = match LocalStorage::get::<EventWeekday>(LOCALSTORAGE_WEEK_START) {
                            Ok(w) => match w {
                                EventWeekday::Sunday => None,
                                w => Some(weekday_from_jiff(weekday_to_jiff(w).next())),
                            },
                            Err(_) => Some(EventWeekday::Monday),
                        };
                        match next {
                            Some(w) => {
                                if let Err(e) = LocalStorage::set(LOCALSTORAGE_WEEK_START, w) {
                                    state().log.log(&format!("Error storing week start: {}", e));
                                }
                            },
                            None => LocalStorage::delete(LOCALSTORAGE_WEEK_START),
                        }
                        button.ref_text(&week_start_text());
                    }
                });
                button
            },
        ],
    }).root;
}
