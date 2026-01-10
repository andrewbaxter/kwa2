use {
    crate::state::{
        ministate_octothorpe,
        Ministate,
    },
    rooting::El,
    crate::js::style_export,
};

pub fn build() -> El {
    return style_export::cont_page_menu(style_export::ContPageMenuArgs {
        head_bar: style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                text: format!("Add channel"),
                link: None,
            }).root,
            right: None,
        }).root,
        children: vec![
            //. .
            style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Join (URL)"),
                link: ministate_octothorpe(&Ministate::ChannelJoinUrl),
                image: None,
            }).root,
            style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("New group"),
                link: ministate_octothorpe(&Ministate::ChannelGroupNew),
                image: None,
            }).root,
            style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("New channel"),
                link: ministate_octothorpe(&Ministate::ChannelNew),
                image: None,
            }).root,
        ],
    }).root;
}
