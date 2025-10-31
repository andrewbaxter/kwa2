use {
    crate::libmain::{
        api::req_post_json,
        formutil::build_form,
        localdata::{
            self,
            get_stored_api_channelgroups,
            req_api_channelgroups,
            LocalChannelGroup,
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
        shared::{
            ChannelGroupId,
        },
    },
    wasm::js::{
        el_async,
        style_export,
    },
};

pub fn build1(eg: EventGraph, value: LocalChannelGroup) -> El {
    return build_form(
        format!("Delete group"),
        Ministate::ChannelGroup(value.res.id.clone()),
        el("div"),
        vec![
            style_export::leaf_form_text(
                style_export::LeafFormTextArgs {
                    text: format!("Are you sure you want to delete group [{}]", value.res.memo_short),
                },
            ).root
        ],
        async move |_idem| {
            req_post_json(&state().env.base_url, c2s::ChannelGroupDelete { id: value.res.id.clone() }).await?;
            localdata::delete_channelgroup(value.res.clone()).await;
            eg.event(|pc| {
                goto_replace_ministate(pc, &state().log, &Ministate::Top);
            }).unwrap();
            return Ok(());
        },
    );
}

pub fn build(pc: &mut ProcessingContext, id: &ChannelGroupId) -> El {
    match get_stored_api_channelgroups(Some(id)).into_iter().find(|x| x.res.id == *id) {
        Some(value) => {
            return build1(pc.eg(), value);
        },
        None => {
            return el_async({
                let eg = pc.eg();
                let id = id.clone();
                async move {
                    let Some(value) =
                        req_api_channelgroups(Some(&id)).await?.into_iter().find(|x| x.res.id == id) else {
                            return Err(format!("Could not find channel group [{:?}]", id));
                        };
                    return Ok(vec![build1(eg.clone(), value)]);
                }
            });
        },
    }
}
