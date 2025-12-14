use {
    crate::{
        js::style_export,
        localdata::get_or_req_api_identity,
        pageutil::{
            build_nol_menu,
            LazyPage,
        },
        state::{
            ministate_octothorpe,
            Ministate,
        },
    },
    rooting::El,
    spaghettinuum::interface::identity::Identity,
};

pub fn build(identity: &Identity) -> El {
    return build_nol_menu(&Ministate::Identities, get_or_req_api_identity(identity, true), |local| LazyPage {
        center: style_export::leaf_head_bar_center(style_export::LeafHeadBarCenterArgs {
            text: local.res.memo_short.clone(),
            link: None,
        }).root,
        body: vec![
            //. .
            style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Edit"),
                link: ministate_octothorpe(&Ministate::IdentityEdit(local.res.id.clone())),
            }).root,
            style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Invitations"),
                link: ministate_octothorpe(&Ministate::IdentityInvites(local.res.id.clone())),
            }).root,
            style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                text: format!("Delete"),
                link: ministate_octothorpe(&Ministate::IdentityDelete(local.res.id.clone())),
            }).root,
        ],
    });
}
