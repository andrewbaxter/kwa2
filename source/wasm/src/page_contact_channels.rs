use {
    crate::{
        api::{
            channel_image,
            req_get,
        },
        async_::bg_val,
        js::style_export,
        localdata::{
            req_api_channels,
            NowOrLater,
        },
        pageutil::build_nol_menu_title,
        state::{
            Ministate,
            MinistateChannel,
            MinistateContact,
            ministate_octothorpe,
        },
    },
    lunk::ProcessingContext,
    rooting::El,
    shared::interface::wire::c2s,
    std::collections::HashMap,
};

pub fn build(pc: &mut ProcessingContext, s: &MinistateContact) -> El {
    return build_nol_menu_title(pc, &Ministate::Contact(s.clone()), "Channels", NowOrLater::Later(bg_val({
        let identity = s.identity.clone();
        async move {
            let contact_channels = req_get(c2s::GetContactChannels { identity: identity }).await?;
            let local_channels =
                req_api_channels(None)
                    .await?
                    .into_iter()
                    .map(|c| (c.res.id.clone(), c.res))
                    .collect::<HashMap<_, _>>();
            let mut out = vec![];
            for c in contact_channels {
                let text;
                let image;
                match local_channels.get(&c.channel) {
                    Some(local) => {
                        text = local.name.clone();
                        image = channel_image(&c.channel, &local.portrait);
                    },
                    None => {
                        text = format!("{} / {}", c.channel.identity, c.channel.channel.0);
                        image = channel_image(&c.channel, &None);
                    },
                }
                out.push(style_export::leaf_menu_link(style_export::LeafMenuLinkArgs {
                    text: text,
                    link: ministate_octothorpe(&Ministate::Channel(MinistateChannel {
                        id: c.channel.clone(),
                        own_identity: c.own_identity.clone(),
                        reset_id: None,
                    })),
                    image: image.image,
                    generated_image: image.generated_image,
                }).root);
            }
            if out.is_empty() {
                out.push(
                    style_export::leaf_calendar_day_empty(
                        style_export::LeafCalendarDayEmptyArgs { text: format!("No known channels") },
                    ).root,
                );
            }
            return Ok(Some(out));
        }
    })), |children| style_export::cont_group(style_export::ContGroupArgs { children: children }).root);
}
