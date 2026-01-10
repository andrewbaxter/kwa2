use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_channelinvite,
        pageutil::{
            LazyPage, build_nol_menu
        },
        state::{
            Ministate, MinistateChannelInvite, ministate_octothorpe
        },
    }, lunk::ProcessingContext, rooting::El, shared::interface::shared::{
        ChannelInviteId,
        QualifiedChannelId,
    }
};

pub fn build(pc: &mut ProcessingContext, channel: &QualifiedChannelId, id: &ChannelInviteId) -> El {
    return build_nol_menu(
        //. .
        pc,&Ministate::ChannelInvites(channel.clone()), get_or_req_api_channelinvite(id, true), {
        let channel = channel.clone();
        move |local| LazyPage {
            center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                text: local.res.memo_short.clone(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::ChannelInviteEdit(MinistateChannelInvite {
                        channel: channel.clone(),
                        invite: local.res.id.clone(),
                    })),
                    image: None,
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ChannelInviteDelete(MinistateChannelInvite {
                        channel: channel.clone(),
                        invite: local.res.id.clone(),
                    })),
                    image: None,
                }).root,
            ],
        }
    });
}
