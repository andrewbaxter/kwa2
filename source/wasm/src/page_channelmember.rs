use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_contact_opt,
        pageutil::{
            LazyPage,
            build_nol_menu,
        },
        state::{
            Ministate,
            MinistateChannelMember,
            MinistateContact,
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
        get_or_req_api_contact_opt(&s.identity, true),
        {
            let s = s.clone();
            move |local| LazyPage {
                center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                    text: match &local {
                        Some(local) if !local.res.name.is_empty() => local.res.name.clone(),
                        _ => s.identity.to_string(),
                    },
                    link: None,
                }).root,
                body: vec![
                    //. .
                    style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: format!("Profile"),
                        link: ministate_octothorpe(&Ministate::Contact(MinistateContact {
                            identity: s.identity.clone(),
                            channel: Some(s.channel.clone()),
                            back: Box::new(Ministate::ChannelMember(s.clone())),
                        })),
                        image: None,
                        generated_image: None,
                    }).root,
                    style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: format!("Edit"),
                        link: ministate_octothorpe(&Ministate::ChannelMemberEdit(s.clone())),
                        image: None,
                        generated_image: None,
                    }).root,
                    style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                        text: format!("Delete"),
                        link: ministate_octothorpe(&Ministate::ChannelMemberDelete(s.clone())),
                        image: None,
                        generated_image: None,
                    }).root,
                ],
            }
        },
    );
}
