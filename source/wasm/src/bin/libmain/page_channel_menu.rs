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
    shared::interface::wire::shared::{
        QualifiedChannelId,
    },
    wasm::js::{
        el_async,
        style_export,
    },
};

fn build1(local: LocalChannel) -> El {
    let bar = style_export::cont_menu_bar(style_export::ContMenuBarArgs {
        back_link: ministate_octothorpe(&Ministate::Channel(local.res.id.clone())),
        text: local.res.memo_short.clone(),
        center_link: None,
        right: None,
    });
    return style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        bar.root,
        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: format!("Edit"),
            link: ministate_octothorpe(&Ministate::ChannelEdit(local.res.id.clone())),
        }).root,
        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: format!("Delete"),
            link: ministate_octothorpe(&Ministate::ChannelDelete(local.res.id.clone())),
        }).root,
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
