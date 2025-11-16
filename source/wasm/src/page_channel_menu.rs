use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_channel,
        pageutil::{
            build_nol_menu,
            LazyPage,
        },
        state::{
            ministate_octothorpe,
            Ministate, MinistateChannel,
        },
    },
    rooting::El,
    shared::interface::shared::QualifiedChannelId,
};

pub fn build(id: &QualifiedChannelId, ) -> El {
    return build_nol_menu(&Ministate::Channel(MinistateChannel{
        id:id.clone(),
        reset_id: None,
    }), get_or_req_api_channel(id, true), {
        move |local| LazyPage {
            center: style_export::leaf_menu_bar_center(style_export::LeafMenuBarCenterArgs {
                text: local.res.memo_short.clone(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::ChannelEdit(local.res.id.clone())),
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ChannelDelete(local.res.id.clone())),
                }).root
            ],
        }
    });
}
