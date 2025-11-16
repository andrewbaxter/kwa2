use {
    crate::{
        chat_entry::{
            ChatEntryMessage,
            ChatEntryMessageInternal,
        },
        js::style_export,
    },
    lunk::{
        link,
        ProcessingContext,
    },
    rooting::El,
};

pub fn build_chat_entry_message(pc: &mut ProcessingContext, m: &ChatEntryMessage) -> El {
    let out = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    out.ref_own(|root| link!((pc = pc), (int = m.internal.clone()), (), (root = root.weak()) {
        let root = root.upgrade()?;
        root.ref_clear();
        match &*int.borrow() {
            ChatEntryMessageInternal::Obviated => {
                root.ref_push(style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root);
            },
            ChatEntryMessageInternal::Deleted => {
                root.ref_push(style_export::leaf_chat_entry_message_deleted().root);
            },
            ChatEntryMessageInternal::Message(m) => {
                let m_el = style_export::leaf_chat_entry_message();
                m_el.root.ref_own(|_| link!((_pc = pc), (body = m.body.clone()), (), (body_el = m_el.body.clone()) {
                    body_el.ref_text(&*body.borrow());
                }));
                root.ref_push(m_el.root);
            },
        }
    }));
    return out;
}
