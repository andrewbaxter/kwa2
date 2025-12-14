use {
    crate::{
        js::style_export,
        localdata::greq_api_identityinvites,
        pageutil::{
            build_nol_menu,
            LazyPage,
        },
        state::{
            ministate_octothorpe,
            Ministate,
            MinistateIdentityInvite,
        },
    },
    rooting::El,
    shared::interface::shared::IdentityInviteId,
    spaghettinuum::interface::identity::Identity,
};

pub fn build(identity: &Identity, id: &IdentityInviteId) -> El {
    return build_nol_menu(&Ministate::IdentityInvites(identity.clone()), greq_api_identityinvites(id, true), {
        let identity = identity.clone();
        move |local| LazyPage {
            center: style_export::leaf_head_bar_center(style_export::LeafHeadBarCenterArgs {
                text: local.res.memo_short.clone(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_code(style_export::LeafMenuCodeArgs { text: local.res.token.token.0.clone() }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::IdentityInviteEdit(MinistateIdentityInvite {
                        identity: identity.clone(),
                        invite: local.res.id.clone(),
                    })),
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::IdentityInviteDelete(MinistateIdentityInvite {
                        identity: identity.clone(),
                        invite: local.res.id.clone(),
                    })),
                }).root,
            ],
        }
    });
}
