use {
    crate::{
        js::style_export,
        localdata::greq_api_identityinvites,
        pageutil::{
            LazyPage,
            build_nol_menu,
            build_url_menu_items,
        },
        state::{
            Ministate,
            MinistateIdentityInvite,
            ministate_octothorpe,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    shared::interface::{
        shared::IdentityInviteId,
        wire::kwaurl::{
            KwaUrl,
            KwaUrlIdentityInvite,
        },
    },
    spaghettinuum::interface::identity::Identity,
};

pub fn build(pc: &mut ProcessingContext, identity: &Identity, id: &IdentityInviteId) -> El {
    return build_nol_menu(
        //. .
        pc,
        &Ministate::IdentityInvites(identity.clone()),
        greq_api_identityinvites(id, true),
        {
            let identity = identity.clone();
            move |local| LazyPage {
                center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                    text: local.res.name.clone(),
                    link: None,
                }).root,
                body: build_url_menu_items(&KwaUrl::IdentityInvite(KwaUrlIdentityInvite {
                    identity: local.res.token.identity.clone(),
                    code: local.res.token.token.0.clone(),
                })).into_iter().chain([style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::IdentityInviteEdit(MinistateIdentityInvite {
                        identity: identity.clone(),
                        invite: local.res.id.clone(),
                    })),
                    image: None,
                    generated_image: None,
                }).root, style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::IdentityInviteDelete(MinistateIdentityInvite {
                        identity: identity.clone(),
                        invite: local.res.id.clone(),
                    })),
                    image: None,
                    generated_image: None,
                }).root]).collect(),
            }
        },
    );
}
