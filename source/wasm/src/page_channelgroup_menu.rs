use {
    crate::{
        js::style_export,
        pageutil::{
            LazyPage,
            build_nol_menu,
        },
        state::{
            Ministate,
            MinistateChannelGroup,
            get_or_req_channelgroup,
            ministate_octothorpe,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    shared::interface::shared::ChannelGroupId,
};

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId) -> El {
    return build_nol_menu(
        //. .
        pc,&Ministate::ChannelGroup(MinistateChannelGroup {
        id: id.clone(),
        reset_id: None,
    }), get_or_req_channelgroup(&pc.eg(), id, true), {
        move |local| LazyPage {
            center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                text: local.memo_short.get(),
                link: None,
            }).root,
            body: vec![
                //. .
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Edit"),
                    link: ministate_octothorpe(&Ministate::ChannelGroupEdit(local.id.clone())),
                    image: None,
                }).root,
                style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: format!("Delete"),
                    link: ministate_octothorpe(&Ministate::ChannelGroupDelete(local.id.clone())),
                    image: None,
                }).root,
            ],
        }
    });
}
