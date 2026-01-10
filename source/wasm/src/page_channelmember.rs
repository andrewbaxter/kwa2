use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_contact,
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
    lunk::ProcessingContext,
    rooting::El,
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateChannelMember) -> El {
    return build_nol_menu(
        //. .
        pc,
        &Ministate::ChannelMembers(s.channel.clone()),
        get_or_req_api_contact(&s.identity, true),
        {
            let channel = s.channel.clone();
            move |local| LazyPage {
                center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
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
                        image: None,
                    }).root,
                    style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: format!("Delete"),
                        link: ministate_octothorpe(&Ministate::ChannelMemberDelete(MinistateChannelMember {
                            channel: channel.clone(),
                            identity: local.res.id.clone(),
                        })),
                        image: None,
                    }).root,
                ],
            }
        },
    );
}
