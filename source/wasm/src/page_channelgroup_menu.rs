use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_channelgroup,
        pageutil::{
            build_nol_menu,
            LazyPage,
        },
        state::{
            ministate_octothorpe,
            Ministate,
            MinistateChannelGroup,
        },
    },
    rooting::El,
    shared::interface::shared::ChannelGroupId,
};

pub fn build(id: &ChannelGroupId) -> El {
    return build_nol_menu(&Ministate::ChannelGroup(MinistateChannelGroup {
        id: id.clone(),
        reset_id: None,
    }), get_or_req_api_channelgroup(id, true), {
        move |local| LazyPage {
            center: style_export::leaf_head_bar_center(style_export::LeafHeadBarCenterArgs {
                text: local.res.memo_short.clone(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::ChannelGroupEdit(local.res.id.clone())),
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ChannelGroupDelete(local.res.id.clone())),
                }).root,
            ],
        }
    });
}
