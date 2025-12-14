use {
    crate::state::{
        ministate_octothorpe,
        Ministate,
    },
    rooting::El,
    crate::js::style_export,
};

pub fn build() -> El {
    return style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        style_export::cont_head_bar(style_export::ContHeadBarArgs {
            back_link: ministate_octothorpe(&Ministate::Top),
            center: style_export::leaf_head_bar_center(style_export::LeafHeadBarCenterArgs {
                text: format!("Add channel"),
                link: None,
            }).root,
            right: None,
        }).root,
        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: format!("Join (URL)"),
            link: ministate_octothorpe(&Ministate::ChannelJoinUrl),
        }).root,
        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: format!("New group"),
            link: ministate_octothorpe(&Ministate::ChannelGroupNew),
        }).root,
        style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
            text: format!("New channel"),
            link: ministate_octothorpe(&Ministate::ChannelNew),
        }).root,
    ] }).root;
}
