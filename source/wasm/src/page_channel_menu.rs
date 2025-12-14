use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_channel,
        pageutil::{
            LazyPage,
            build_nol_menu,
        },
        state::{
            Ministate,
            MinistateChannel,
            MinistateChannelSub,
            ministate_octothorpe,
        },
    },
    rooting::El,
};

pub fn build(s: &MinistateChannelSub) -> El {
    return build_nol_menu(&Ministate::Channel(MinistateChannel {
        id: s.id.clone(),
        own_identity: s.own_identity.clone(),
        reset_id: None,
    }), get_or_req_api_channel(&s.id, true), {
        let sender = s.own_identity.clone();
        move |local| {
            let mut children = vec![];
            children.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Edit"),
                link: ministate_octothorpe(&Ministate::ChannelEdit(MinistateChannelSub {
                    id: local.res.id.clone(),
                    own_identity: sender.clone(),
                })),
            }).root);
            if local.res.id.identity == sender {
                children.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Members"),
                    link: ministate_octothorpe(&Ministate::ChannelMembers(local.res.id.clone())),
                }).root);
            }
            children.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Delete"),
                link: ministate_octothorpe(&Ministate::ChannelDelete(MinistateChannelSub {
                    id: local.res.id.clone(),
                    own_identity: sender.clone(),
                })),
            }).root);
            return LazyPage {
                center: style_export::leaf_head_bar_center(style_export::LeafHeadBarCenterArgs {
                    text: local.res.memo_short.clone(),
                    link: None,
                }).root,
                body: children,
            };
        }
    });
}
