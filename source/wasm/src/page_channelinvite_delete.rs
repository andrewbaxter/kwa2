use {
    crate::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
            get_stored_api_channelinvites,
            req_api_channelinvites,
            LocalChannelInvite,
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
        shared::ChannelInviteId,
    },
    crate::js::{
        el_async,
        style_export,
    },
};

pub fn build_page1(eg: EventGraph, value: LocalChannelInvite) -> El {
    return build_form(
        format!("Delete invite"),
        Ministate::ChannelInvite(value.res.id.clone()),
        el("div"),
        vec![
            style_export::leaf_form_text(
                style_export::LeafFormTextArgs {
                    text: format!("Are you sure you want to delete invite [{}]", value.res.memo_short),
                },
            ).root
        ],
        async move |_idem| {
            req_post_json(&state().env.base_url, c2s::ChannelInviteDelete { id: value.res.id.clone() }).await?;
            localdata::delete_channelinvite(value.res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(
                    pc,
                    &state().log,
                    &Ministate::ChannelInvites(value.res.token.channel.clone()),
                );
            }).unwrap();
            return Ok(());
        },
    );
}

pub fn build(pc: &mut ProcessingContext, id: &ChannelInviteId) -> El {
    match get_stored_api_channelinvites(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(local) => {
            return build_page1(pc.eg(), local);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let id = id.clone();
                async move {
                    let Some(local) =
                        req_api_channelinvites(Some(&id)).await?.into_iter().find(|x| x.res.id == id) else {
                            return Err(format!("Could not find channel invite [{:?}]", id));
                        };
                    return Ok(vec![build_page1(eg.clone(), local)]);
                }
            });
        },
    }
}
