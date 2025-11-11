use {
    crate::{
        api::{
            req_get,
            req_post_json,
        },
        js::{
            el_async,
            el_menu_button,
            style_export,
            Engine,
        },
        state::{
            ministate_octothorpe,
            state,
            Ministate,
        },
    },
    flowcontrol::{
        shed,
        ta_return,
    },
    js_sys::JSON,
    rooting::El,
    shared::interface::wire::c2s,
    wasm_bindgen::{
        JsValue,
    },
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        Notification,
        NotificationPermission,
        PushManager,
        PushSubscription,
        PushSubscriptionOptionsInit,
    },
};

async fn register(pm: PushManager) -> Result<(), String> {
    match NotificationPermission::from_js_value(
        &JsFuture::from(
            Notification
            ::request_permission().map_err(
                |e| format!("Error requesting permissions for notifications: {:?}", e.as_string()),
            )?,
        )
            .await
            .map_err(|e| format!("Error requesting permissions for notifications: {:?}", e.as_string()))?,
    ).expect("Notification permission result didn't match permission struct") {
        NotificationPermission::Granted => { },
        _ => return Err(format!("Notification permission denied")),
    }
    let key = req_get(&state().env.base_url, c2s::NotificationServerKey).await?;
    let sub =
        PushSubscription::from(
            JsFuture::from(pm.subscribe_with_options(&{
                let o = PushSubscriptionOptionsInit::new();
                o.set_user_visible_only(true);
                o.set_application_server_key(&JsValue::from(key));
                o
            }).map_err(|e| format!("Error setting up subscription for notifications: {:?}", e.as_string()))?)
                .await
                .map_err(
                    |e| format!("Error setting up subscription for notifications (async): {:?}", e.as_string()),
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
    return Ok(());
}

pub fn build() -> El {
    return style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        style_export::cont_menu_bar(style_export::ContMenuBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            center: style_export::leaf_menu_bar_center(style_export::LeafMenuBarCenterArgs {
                text: format!("Settings"),
                link: None,
            }).root,
            right: None,
        }).root,
        el_async(async move {
            ta_return!(Vec < El >, String);
            if state().env.engine == Some(Engine::IosSafari) && !state().env.pwa {
                return Ok(
                    vec![
                        style_export::leaf_err_block(
                            style_export::LeafErrBlockArgs {
                                data: format!("This app must be added to the home screen to enable notifications"),
                            },
                        ).root
                    ],
                );
            }
            let sw = state().service_worker.get().await?;
            let pm = sw.push_manager().map_err(|e| format!("{:?}", e.as_string()))?;
            shed!{
                'no _;
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
                return Ok(vec![el_menu_button("Re-enable notifications".to_string(), async move || {
                    register(pm.clone()).await?;
                    return Ok(());
                })]);
            }
            return Ok(vec![el_menu_button("Enable notifications".to_string(), async move || {
                register(pm.clone()).await?;
                return Ok(());
            })]);
        }),
    ] }).root;
}
