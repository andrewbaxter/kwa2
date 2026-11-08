use {
    crate::{
        api::req_get,
        js::{
            el_async,
            style_export,
        },
        state::{
            MinistateMessageHistory,
            ministate_octothorpe,
        },
    },
    flowcontrol::ta_return,
    lunk::ProcessingContext,
    rooting::El,
    shared::interface::wire::c2s::{
        self,
        MessageRevision,
    },
    similar::{
        ChangeTag,
        TextDiff,
    },
};

fn build_spans(segments: &Vec<(bool, String)>) -> Vec<El> {
    let mut out = vec![];
    for (changed, text) in segments {
        out.push(style_export::leaf_message_history_diff_text(style_export::LeafMessageHistoryDiffTextArgs {
            text: text.clone(),
            changed: *changed,
        }).root);
    }
    return out;
}

pub fn build(_pc: &mut ProcessingContext, m: &MinistateMessageHistory) -> El {
    let body = el_async({
        let id = m.id.clone();
        async move {
            ta_return!(Vec < El >, String);
            let Some(revisions) = req_get(c2s::MessageHistory { id: id }).await? else {
                return Err(format!("Message not found"));
            };
            let mut out = vec![];
            let mut previous: Option<&MessageRevision> = None;
            for revision in &revisions {
                match previous {
                    None => {
                        out.push(
                            style_export::cont_message_history_revision_original(
                                style_export::ContMessageHistoryRevisionOriginalArgs {
                                    date: revision.receive_time.to_string(),
                                    text: revision.body.clone(),
                                },
                            ).root,
                        );
                    },
                    Some(previous) => {
                        let diff = TextDiff::from_lines(&previous.body, &revision.body);
                        let mut cells = vec![];
                        for op in diff.ops() {
                            let mut removed = vec![];
                            let mut added = vec![];
                            for change in diff.iter_inline_changes(op) {
                                let tag = change.tag();
                                let segments =
                                    change
                                        .iter_strings_lossy()
                                        .map(
                                            |(emphasized, text)| (
                                                emphasized,
                                                text.trim_end_matches('\n').to_string(),
                                            ),
                                        )
                                        .collect::<Vec<_>>();
                                match tag {
                                    ChangeTag::Equal => {
                                        cells.push(
                                            style_export::cont_message_history_diff_line_context(
                                                style_export::ContMessageHistoryDiffLineContextArgs {
                                                    children: build_spans(&segments),
                                                },
                                            ).root,
                                        );
                                        cells.push(
                                            style_export::cont_message_history_diff_line_context(
                                                style_export::ContMessageHistoryDiffLineContextArgs {
                                                    children: build_spans(&segments),
                                                },
                                            ).root,
                                        );
                                    },
                                    ChangeTag::Delete => {
                                        removed.push(segments);
                                    },
                                    ChangeTag::Insert => {
                                        added.push(segments);
                                    },
                                }
                            }
                            let mut removed = removed.into_iter();
                            let mut added = added.into_iter();
                            loop {
                                let left = removed.next();
                                let right = added.next();
                                if left.is_none() && right.is_none() {
                                    break;
                                }
                                cells.push(match left {
                                    Some(segments) => style_export::cont_message_history_diff_line_removed(
                                        style_export::ContMessageHistoryDiffLineRemovedArgs {
                                            children: build_spans(&segments),
                                        },
                                    ).root,
                                    None => style_export::leaf_message_history_diff_line_empty().root,
                                });
                                cells.push(match right {
                                    Some(segments) => style_export::cont_message_history_diff_line_added(
                                        style_export::ContMessageHistoryDiffLineAddedArgs {
                                            children: build_spans(&segments),
                                        },
                                    ).root,
                                    None => style_export::leaf_message_history_diff_line_empty().root,
                                });
                            }
                        }
                        out.push(
                            style_export::cont_message_history_revision_edit(
                                style_export::ContMessageHistoryRevisionEditArgs {
                                    date: revision.receive_time.to_string(),
                                    cells: cells,
                                },
                            ).root,
                        );
                    },
                }
                previous = Some(revision);
            }
            return Ok(out);
        }
    });
    return style_export::cont_page_message_history(style_export::ContPageMessageHistoryArgs {
        head_bar: style_export::cont_nonchat_head_bar(style_export::ContNonchatHeadBarArgs {
            back_link: ministate_octothorpe(&m.back),
            center: style_export::leaf_nonchat_head_bar_center(style_export::LeafNonchatHeadBarCenterArgs {
                text: format!("History"),
                link: None,
            }).root,
            right: None,
        }).root,
        children: vec![body],
    }).root;
}
