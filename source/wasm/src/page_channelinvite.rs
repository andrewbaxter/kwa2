use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_channelinvite,
        pageutil::{
            build_nol_menu,
            LazyPage,
        },
        state::{
            ministate_octothorpe,
            Ministate,
            MinistateChannel,
            MinistateChannelInvite,
        },
    },
    rooting::El,
    shared::interface::shared::{
        ChannelInviteId,
        QualifiedChannelId,
        QualifiedMessageId,
    },
};

pub fn build(channel: &QualifiedChannelId, id: &ChannelInviteId, reset_id: &Option<QualifiedMessageId>) -> El {
    return build_nol_menu(&Ministate::ChannelInvites(MinistateChannel {
        channel: channel.clone(),
        reset: reset_id.clone(),
    }), get_or_req_api_channelinvite(id, true), {
        let reset_id = reset_id.clone();
        let channel = channel.clone();
        move |local| LazyPage {
            center: style_export::leaf_menu_bar_center(style_export::LeafMenuBarCenterArgs {
                text: local.res.memo_short.clone(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::ChannelInviteEdit(MinistateChannelInvite {
                        channel: channel.clone(),
                        reset: reset_id.clone(),
                        invite: local.res.id.clone(),
                    })),
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ChannelInviteDelete(MinistateChannelInvite {
                        channel: channel.clone(),
                        reset: reset_id.clone(),
                        invite: local.res.id.clone(),
                    })),
                }).root,
            ],
        }
    });
}
