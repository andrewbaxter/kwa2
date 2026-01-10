use {
    crate::{
        api::portrait_url,
        chat_entry::{
            ChatEntryMessage,
            ChatEntryMessageInternal,
        },
        js::style_export,
        localdata,
    },
    lunk::{
        ProcessingContext,
        link,
    },
    rooting::El,
};

pub fn build_chat_entry_message(pc: &mut ProcessingContext, m: &ChatEntryMessage) -> El {
    let out = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let left = !localdata::get_stored_api_identities(None).into_iter().any(|x| x.res.id == m.sender);
    out.ref_own(
        |root| link!(
            (pc = pc),
            (int = m.internal.clone()),
            (),
            (root = root.weak(), left = left, sender = m.sender.clone()) {
                let root = root.upgrade()?;
                root.ref_clear();
                match &*int.borrow() {
                    ChatEntryMessageInternal::Obviated => {
                        root.ref_push(
                            style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root
                        );
                    },
                    ChatEntryMessageInternal::Deleted => {
                        root.ref_push(
                            style_export::cont_chat_entry_mode_deleted(
                                style_export::ContChatEntryModeDeletedArgs { left: *left }
                            ).root
                        );
                    },
                    ChatEntryMessageInternal::Message(m) => {
                        let m_el =
                            style_export::cont_chat_entry_mode_message(style_export::ContChatEntryModeMessageArgs {
                                left: *left,
                                date: m.time.to_string(),
                                image: portrait_url(&sender)
                            });
                        m_el
                            .root
                            .ref_own(|_| link!((_pc = pc), (body = m.body.clone()), (), (body_el = m_el.body.clone()) {
                                body_el.ref_text(&*body.borrow());
                            }));
                        root.ref_push(m_el.root);
                    },
                }
            }
        ),
    );
    return out;
}
