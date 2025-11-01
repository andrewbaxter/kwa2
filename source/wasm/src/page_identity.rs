use {
    crate::{
        localdata::{
            get_stored_api_identities,
            req_api_identities,
            LocalIdentity,
        },
        state::{
            ministate_octothorpe,
            Ministate,
        },
    },
    rooting::El,
    spaghettinuum::interface::identity::Identity,
    crate::js::{
        el_async,
        style_export,
    },
};

fn build1(local: LocalIdentity) -> El {
    let bar = style_export::cont_menu_bar(style_export::ContMenuBarArgs {
        back_link: ministate_octothorpe(&Ministate::Identities),
        text: local.res.memo_short.clone(),
        center_link: None,
        right: None,
    });
    return style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![
        //. .
        bar.root,
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
    ] }).root;
}

pub fn build(identity: &Identity) -> El {
    match get_stored_api_identities(Some(identity)).into_iter().find(|x| x.res.id == *identity) {
        Some(local) => {
            return build1(local);
        },
        None => {
            return el_async({
                let identity = identity.clone();
                async move {
                    let Some(local) =
                        req_api_identities(Some(&identity))
                            .await?
                            .into_iter()
                            .find(|x| x.res.id == identity) else {
                            return Err(format!("Could not find identity [{}]", identity));
                        };
                    return Ok(vec![build1(local)]);
                }
            });
        },
    }
}
