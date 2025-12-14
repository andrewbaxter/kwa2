use {
    crate::{
        js::style_export,
        localdata::{
            get_or_req_api_contact,
        },
        pageutil::{
            LazyPage,
            build_nol_menu,
        },
        state::{
            Ministate,
            MinistateChannelMember,
            ministate_octothorpe,
        },
    },
    rooting::El,
};

pub fn build(s: &MinistateChannelMember) -> El {
    return build_nol_menu(&Ministate::ChannelMembers(s.channel.clone()), get_or_req_api_contact(&s.identity, true), {
        let channel = s.channel.clone();
        move |local| LazyPage {
            center: style_export::leaf_head_bar_center(style_export::LeafHeadBarCenterArgs {
                text: local.res.memo_short.clone(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::ChannelMemberEdit(MinistateChannelMember {
                        channel: channel.clone(),
                        identity: local.res.id.clone(),
                    })),
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ChannelMemberDelete(MinistateChannelMember {
                        channel: channel.clone(),
                        identity: local.res.id.clone(),
                    })),
                }).root,
            ],
        }
    });
}
