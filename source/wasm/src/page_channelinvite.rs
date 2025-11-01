use {
    crate::{
        localdata::{
            get_stored_api_channelinvites,
            req_api_channelinvites,
            LocalChannelInvite,
        },
        state::{
            ministate_octothorpe,
            Ministate,
        },
    },
    rooting::El,
    shared::interface::wire::shared::{
        ChannelInviteId,
    },
    crate::js::{
        el_async,
        style_export,
    },
};

fn build1(local: LocalChannelInvite) -> El {
    let bar = style_export::cont_menu_bar(style_export::ContMenuBarArgs {
        back_link: ministate_octothorpe(&Ministate::ChannelInvites(local.res.token.channel.clone())),
        text: local.res.memo_short.clone(),
        center_link: None,
        right: None,
    });
    return style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        bar.root,
        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: format!("Edit"),
            link: ministate_octothorpe(&Ministate::ChannelInviteEdit(local.res.id.clone())),
        }).root,
        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: format!("Delete"),
            link: ministate_octothorpe(&Ministate::ChannelInviteDelete(local.res.id.clone())),
        }).root,
    ] }).root;
}

pub fn build(id: &ChannelInviteId) -> El {
    match get_stored_api_channelinvites(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(local) => {
            return build1(local);
        },
        None => {
            return el_async({
                let id = id.clone();
                async move {
                    let Some(local) =
                        req_api_channelinvites(Some(&id)).await?.into_iter().find(|x| x.res.id == id) else {
                            return Err(format!("Could not find invite [{}]", id.0));
                        };
                    return Ok(vec![build1(local)]);
                }
            });
        },
    }
}
