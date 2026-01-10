use {
    crate::{
        js::style_export,
        pageutil::{
            LazyPage,
            build_nol_menu,
        },
        state::{
            Ministate,
            MinistateChannel,
            MinistateChannelSub,
            get_or_req_channel,
            ministate_octothorpe,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelSub) -> El {
    return build_nol_menu(
        //. .
        pc,&Ministate::Channel(MinistateChannel {
        id: s.id.clone(),
        own_identity: s.own_identity.clone(),
        reset_id: None,
    }), get_or_req_channel(&pc.eg(), &s.id, true), {
        let sender = s.own_identity.clone();
        move |local| {
            let mut children = vec![];
            children.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Edit"),
                link: ministate_octothorpe(&Ministate::ChannelEdit(MinistateChannelSub {
                    id: local.id.clone(),
                    own_identity: sender.clone(),
                })),
                image: None,
            }).root);
            if local.id.identity == sender {
                children.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Members"),
                    link: ministate_octothorpe(&Ministate::ChannelMembers(local.id.clone())),
                    image: None,
                }).root);
            }
            children.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Delete"),
                link: ministate_octothorpe(&Ministate::ChannelDelete(MinistateChannelSub {
                    id: local.id.clone(),
                    own_identity: sender.clone(),
                })),
                image: None,
            }).root);
            return LazyPage {
                center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                    text: local.memo_short.get(),
                    link: None,
                }).root,
                body: children,
            };
        }
    });
}
