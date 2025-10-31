use {
    crate::libmain::{
        localdata::{
            get_stored_api_channels,
            req_api_channels,
            LocalChannel,
        },
        state::{
            ministate_octothorpe,
            Ministate,
        },
    },
    rooting::El,
    shared::interface::wire::shared::QualifiedChannelId,
    wasm::js::{
        el_async,
        style_export,
    },
};

pub fn build1(local: LocalChannel) -> El {
    return style_export::cont_page_chat(style_export::ContPageChatArgs { children: vec![
        //. .
        style_export::cont_chat_bar(style_export::ContChatBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            text: local.res.memo_short.clone(),
            center_link: Some(ministate_octothorpe(&Ministate::ChannelMenu(local.res.id.clone()))),
            right: None,
        }).root
    ] }).root;
}

pub fn build(id: &QualifiedChannelId) -> El {
    match get_stored_api_channels(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(local) => {
            return build1(local);
        },
        None => {
            return el_async({
                let id = id.clone();
                async move {
                    let Some(local) =
                        req_api_channels(Some(&id)).await?.into_iter().find(|x| x.res.id == id) else {
                            return Err(format!("Could not find channel [{:?}]", id));
                        };
                    return Ok(vec![build1(local)]);
                }
            });
        },
    }
}
