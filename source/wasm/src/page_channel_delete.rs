use {
    crate::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
            get_stored_api_channels,
            req_api_channels,
            LocalChannel,
        },
        state::{
            goto_replace_ministate,
            state,
            Ministate,
        },
    },
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::{
        el,
        El,
    },
    shared::interface::wire::{
        c2s::{
            self,
        },
        shared::QualifiedChannelId,
    },
    crate::js::{
        el_async,
        style_export,
    },
};

pub fn build1(eg: EventGraph, value: LocalChannel) -> El {
    return build_form(
        format!("Delete channel"),
        Ministate::Channel(value.res.id.clone()),
        el("div"),
        vec![
            style_export::leaf_form_text(
                style_export::LeafFormTextArgs {
                    text: format!("Are you sure you want to delete channel [{}]", value.res.memo_short),
                },
            ).root
        ],
        async move |_idem| {
            req_post_json(&state().env.base_url, c2s::ChannelDelete { id: value.res.id.clone() }).await?;
            localdata::delete_channel(value.res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Top);
            }).unwrap();
            return Ok(());
        },
    );
}

pub fn build(pc: &mut ProcessingContext, id: &QualifiedChannelId) -> El {
    match get_stored_api_channels(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(value) => {
            return build1(pc.eg(), value);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let channel = id.clone();
                async move {
                    let Some(value) =
                        req_api_channels(Some(&channel)).await?.into_iter().find(|x| x.res.id == channel) else {
                            return Err(format!("Could not find channel [{:?}]", channel));
                        };
                    return Ok(vec![build1(eg.clone(), value)]);
                }
            });
        },
    }
}
