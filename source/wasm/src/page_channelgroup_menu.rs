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
    shared::interface::shared::{
        ChannelGroupId,
        QualifiedMessageId,
    },
};

pub fn build(id: &ChannelGroupId, reset_id: &Option<QualifiedMessageId>) -> El {
    return build_nol_menu(&Ministate::ChannelGroup(MinistateChannelGroup {
        channelgroup: id.clone(),
        reset: reset_id.clone(),
    }), get_or_req_api_channelgroup(id, true), {
        let reset_id = reset_id.clone();
        move |local| LazyPage {
            center: style_export::leaf_menu_bar_center(style_export::LeafMenuBarCenterArgs {
                text: local.res.memo_short.clone(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::ChannelGroupEdit(MinistateChannelGroup {
                        channelgroup: local.res.id.clone(),
                        reset: reset_id.clone(),
                    })),
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ChannelGroupDelete(MinistateChannelGroup {
                        channelgroup: local.res.id.clone(),
                        reset: reset_id.clone(),
                    })),
                }).root,
            ],
        }
    });
}
