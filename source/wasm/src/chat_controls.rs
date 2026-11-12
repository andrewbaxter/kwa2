use {
    crate::{
        blocks::{
            format_size,
            order_blocks,
            LocalBlob,
            ViewBlock,
            ViewFile,
            ViewFileSource,
        },
        calendar::CalChannel,
        camera::open_camera,
        event_editor::{
            default_event,
            event_timeline_text,
            open_event_editor,
        },
        chat::{
            ChatMode,
            ChatModeReplyMessage,
            ChatModeReplyMessageTarget,
            ChatModeReplyMessageTargetOutbox,
            ChatModeTopMessage,
            ChatState,
            ChatState2,
        },
        chat_entry::ChatEntryControls,
        js::{
            ElExt,
            style_export,
        },
        media_viewer::open_media_viewer,
        modal::ModalHandle,
        outbox::{
            new_message_client_id,
            opfs_write_outbox_message,
            OutboxMessageRel,
            OutboxMessageReplyTo,
        },
        state::{
            spawn_log,
            state,
        },
    },
    gloo_events::EventListenerOptions,
    lunk::{
        HistPrim,
        ProcessingContext,
        link,
    },
    rooting::{
        el,
        El,
        scope_any,
    },
    shared::interface::shared::{
        mime_is_media,
        MessageBlockEvent,
        MessageBlockExtension,
        QualifiedChannelId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::{
            Cell,
            RefCell,
        },
        rc::Rc,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        Blob,
        HtmlInputElement,
        KeyboardEvent,
    },
};

struct EditorText {
    root: El,
    plus_slot: El,
    editor_el: El,
    editor: malarkdowney::Editor,
}

struct EditorFile {
    file: ViewFile,
    row: El,
    name: Option<El>,
}

struct EditorEvent {
    event: RefCell<MessageBlockEvent>,
    row: El,
}

struct MessageEditor {
    channel: CalChannel,
    rows: El,
    send: El,
    close: El,
    plus: El,
    toolbox: El,
    toolbox_text: El,
    toolbox_camera: El,
    toolbox_file: El,
    toolbox_event: El,
    toolbox_delete: Option<El>,
    file_input: El,
    toolbox_open: Cell<bool>,
    deleting: Cell<bool>,
    text: RefCell<Option<EditorText>>,
    files: RefCell<Vec<EditorFile>>,
    events: RefCell<Vec<Rc<EditorEvent>>>,
    extensions: RefCell<Vec<(MessageBlockExtension, El)>>,
}

impl MessageEditor {
    fn new(channel: CalChannel, rows: El, send: El, edit: bool, initial: &[ViewBlock]) -> Rc<MessageEditor> {
        let file_input =
            el("input")
                .attr("type", "file")
                .attr("multiple", "")
                .classes(&[&style_export::class_state_hidden().value]);
        let toolbox_text = style_export::leaf_chat_editor_toolbox_button_text().root;
        let toolbox_camera = style_export::leaf_chat_editor_toolbox_button_camera().root;
        let toolbox_file = style_export::leaf_chat_editor_toolbox_button_file().root;
        let toolbox_event = style_export::leaf_chat_editor_toolbox_button_event().root;
        let toolbox_delete = if edit {
            Some(style_export::leaf_chat_editor_toolbox_button_delete().root)
        } else {
            None
        };
        let mut toolbox_children =
            vec![toolbox_text.clone(), toolbox_camera.clone(), toolbox_file.clone(), toolbox_event.clone()];
        toolbox_children.extend(toolbox_delete.clone());
        toolbox_children.push(file_input.clone());
        let editor = Rc::new(MessageEditor {
            channel: channel,
            rows: rows,
            send: send,
            close: style_export::leaf_chat_editor_button_close().root,
            plus: style_export::leaf_chat_editor_button_plus().root,
            toolbox: style_export::cont_chat_editor_toolbox(
                style_export::ContChatEditorToolboxArgs { children: toolbox_children },
            ).root,
            toolbox_text: toolbox_text.clone(),
            toolbox_camera: toolbox_camera.clone(),
            toolbox_file: toolbox_file.clone(),
            toolbox_event: toolbox_event.clone(),
            toolbox_delete: toolbox_delete.clone(),
            file_input: file_input.clone(),
            toolbox_open: Cell::new(false),
            deleting: Cell::new(false),
            text: RefCell::new(None),
            files: RefCell::new(vec![]),
            events: RefCell::new(vec![]),
            extensions: RefCell::new(vec![]),
        });
        if let Some(toolbox_delete) = &toolbox_delete {
            toolbox_delete.ref_on("click", {
                let editor = Rc::downgrade(&editor);
                move |_| {
                    let Some(editor) = editor.upgrade() else {
                        return;
                    };
                    editor.deleting.set(!editor.deleting.get());
                    editor.render();
                }
            });
        }
        editor.plus.ref_on("click", {
            let editor = Rc::downgrade(&editor);
            move |_| {
                let Some(editor) = editor.upgrade() else {
                    return;
                };
                editor.toolbox_open.set(true);
                editor.render();
                editor.focus();
            }
        });
        toolbox_text.ref_on("click", {
            let editor = Rc::downgrade(&editor);
            move |_| {
                let Some(editor) = editor.upgrade() else {
                    return;
                };
                editor.set_text("");
                editor.render();
                editor.focus();
            }
        });
        toolbox_camera.ref_on("click", {
            let editor = Rc::downgrade(&editor);
            move |_| {
                open_camera(Rc::new({
                    let editor = editor.clone();
                    move |blob: Blob| {
                        let Some(editor) = editor.upgrade() else {
                            return;
                        };
                        editor.add_file(ViewFile {
                            mime_type: blob.type_(),
                            size: blob.size() as u64,
                            name: None,
                            source: ViewFileSource::Local(LocalBlob::new(blob)),
                        });
                        editor.render();
                    }
                }));
            }
        });
        toolbox_event.ref_on("click", {
            let editor = Rc::downgrade(&editor);
            move |_| {
                let Some(editor) = editor.upgrade() else {
                    return;
                };
                match default_event() {
                    Ok(event) => editor.add_event(event),
                    Err(e) => {
                        state().log.log(&format!("Error creating event: {}", e));
                        return;
                    },
                }
                editor.render();
            }
        });
        toolbox_file.ref_on("click", {
            let file_input = file_input.weak();
            move |_| {
                let Some(file_input) = file_input.upgrade() else {
                    return;
                };
                file_input.html().click();
            }
        });
        file_input.ref_on("change", {
            let editor = Rc::downgrade(&editor);
            move |_| {
                let Some(editor) = editor.upgrade() else {
                    return;
                };
                let input = editor.file_input.raw().dyn_into::<HtmlInputElement>().unwrap();
                let Some(files) = input.files() else {
                    return;
                };
                for i in 0 .. files.length() {
                    let file = files.get(i).unwrap();
                    let mime_type = if file.type_().is_empty() {
                        "application/octet-stream".to_string()
                    } else {
                        file.type_()
                    };
                    let name = if mime_is_media(&mime_type) {
                        None
                    } else {
                        let full = file.name();
                        Some(match full.rfind('.') {
                            Some(i) if i > 0 => full[..i].to_string(),
                            _ => full,
                        })
                    };
                    editor.add_file(ViewFile {
                        mime_type: mime_type,
                        size: file.size() as u64,
                        name: name,
                        source: ViewFileSource::Local(LocalBlob::new(file.into())),
                    });
                }
                input.set_value("");
                editor.render();
            }
        });
        editor.load(initial);
        return editor;
    }

    fn load(self: &Rc<Self>, blocks: &[ViewBlock]) {
        *self.text.borrow_mut() = None;
        self.files.borrow_mut().clear();
        self.events.borrow_mut().clear();
        self.extensions.borrow_mut().clear();
        self.deleting.set(false);
        let ordered = order_blocks(blocks);
        if !ordered.texts.is_empty() ||
            (ordered.media.is_empty() && ordered.files.is_empty() && ordered.events.is_empty() &&
                ordered.extensions.is_empty()) {
            self.set_text(&ordered.texts.join("\n\n"));
        }
        for event in ordered.events {
            self.add_event(event);
        }
        for file in ordered.media.into_iter().chain(ordered.files.into_iter()) {
            self.add_file(file);
        }
        for extension in ordered.extensions {
            let delete = style_export::leaf_chat_editor_button_delete().root;
            let row = style_export::cont_chat_editor_row(style_export::ContChatEditorRowArgs {
                left: vec![delete.clone()],
                middle: style_export::leaf_chat_entry_message_extension(
                    style_export::LeafChatEntryMessageExtensionArgs {
                        key: extension.key.clone(),
                        json: serde_json::to_string_pretty(&extension.data).unwrap(),
                    },
                ).root,
            }).root;
            delete.ref_on("click", {
                let editor = Rc::downgrade(self);
                let row = row.weak();
                move |_| {
                    let Some(editor) = editor.upgrade() else {
                        return;
                    };
                    let Some(row) = row.upgrade() else {
                        return;
                    };
                    editor.extensions.borrow_mut().retain(|e| e.1.ptr_id() != row.ptr_id());
                    editor.render();
                }
            });
            self.extensions.borrow_mut().push((extension, row));
        }
        self.toolbox_open.set(self.text.borrow().is_none());
        self.render();
    }

    fn set_text(self: &Rc<Self>, text: &str) {
        let (editor_el, editor) = malarkdowney::build(text, malarkdowney::Options { newline_on_enter: false });
        editor_el.ref_on_with_options("keydown", EventListenerOptions::enable_prevent_default(), {
            let send = self.send.weak();
            move |ev| {
                let ev = ev.dyn_ref::<KeyboardEvent>().unwrap();
                if ev.key() != "Enter" || ev.shift_key() || ev.is_composing() {
                    return;
                }
                ev.prevent_default();
                let Some(send) = send.upgrade() else {
                    return;
                };
                send.html().click();
            }
        });
        let text =
            style_export::leaf_chat_editor_text(style_export::LeafChatEditorTextArgs { editor: editor_el.clone() });
        *self.text.borrow_mut() = Some(EditorText {
            root: text.root,
            plus_slot: text.plus_slot,
            editor_el: editor_el,
            editor: editor,
        });
    }

    fn add_event(self: &Rc<Self>, event: MessageBlockEvent) {
        let delete = style_export::leaf_chat_editor_button_delete().root;
        let middle = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
        let row = style_export::cont_chat_editor_row(style_export::ContChatEditorRowArgs {
            left: vec![delete.clone()],
            middle: middle.clone(),
        }).root;
        let entry = Rc::new(EditorEvent {
            event: RefCell::new(event),
            row: row.clone(),
        });

        fn build_middle(editor: &Rc<MessageEditor>, entry: &Rc<EditorEvent>, middle: &El) {
            let timeline_text = match event_timeline_text(&entry.event.borrow()) {
                Ok(t) => t,
                Err(e) => format!("Invalid event time: {}", e),
            };
            let ev =
                style_export::leaf_chat_editor_event(
                    style_export::LeafChatEditorEventArgs { timeline: timeline_text },
                ).root;
            ev.ref_on("click", {
                let editor = Rc::downgrade(editor);
                let entry = Rc::downgrade(entry);
                let middle = middle.weak();
                move |_| {
                    let Some(editor) = editor.upgrade() else {
                        return;
                    };
                    let Some(entry) = entry.upgrade() else {
                        return;
                    };
                    let current = entry.event.borrow().clone();
                    let on_done = Rc::new({
                        let editor = Rc::downgrade(&editor);
                        let entry = Rc::downgrade(&entry);
                        let middle = middle.clone();
                        move |event: MessageBlockEvent| {
                            let Some(editor) = editor.upgrade() else {
                                return;
                            };
                            let Some(entry) = entry.upgrade() else {
                                return;
                            };
                            let Some(middle) = middle.upgrade() else {
                                return;
                            };
                            *entry.event.borrow_mut() = event;
                            build_middle(&editor, &entry, &middle);
                        }
                    });
                    open_event_editor(editor.channel.clone(), &current, on_done);
                }
            });
            middle.ref_clear();
            middle.ref_push(ev);
        }

        build_middle(self, &entry, &middle);
        delete.ref_on("click", {
            let editor = Rc::downgrade(self);
            let row = row.weak();
            move |ev| {
                ev.stop_propagation();
                let Some(editor) = editor.upgrade() else {
                    return;
                };
                let Some(row) = row.upgrade() else {
                    return;
                };
                editor.events.borrow_mut().retain(|e| e.row.ptr_id() != row.ptr_id());
                editor.render();
            }
        });
        self.events.borrow_mut().push(entry);
    }

    fn add_file(self: &Rc<Self>, file: ViewFile) {
        let delete;
        let row;
        let name;
        if file.is_media() {
            let square =
                style_export::leaf_chat_editor_media_square(
                    style_export::LeafChatEditorMediaSquareArgs {
                        content: style_export::leaf_media_thumb(style_export::LeafMediaThumbArgs {
                            src: file.url(true),
                            video: file.is_video(),
                            src_is_video: file.thumbnail_is_video(),
                        }).root,
                    },
                );
            square.root.ref_on("click", {
                let editor = Rc::downgrade(self);
                let square = square.root.weak();
                move |_| {
                    let Some(editor) = editor.upgrade() else {
                        return;
                    };
                    let Some(square) = square.upgrade() else {
                        return;
                    };
                    let media =
                        editor
                            .files
                            .borrow()
                            .iter()
                            .filter(|f| f.file.is_media())
                            .map(|f| f.file.clone())
                            .collect::<Vec<_>>();
                    let index =
                        editor
                            .files
                            .borrow()
                            .iter()
                            .filter(|f| f.file.is_media())
                            .position(|f| f.row.ptr_id() == square.ptr_id())
                            .unwrap();
                    open_media_viewer(media, index);
                }
            });
            delete = square.delete;
            row = square.root;
            name = None;
        } else {
            let file_el = style_export::leaf_chat_editor_file(style_export::LeafChatEditorFileArgs {
                name: file.name.clone().unwrap_or_default(),
                size: format_size(file.size),
            });
            delete = style_export::leaf_chat_editor_button_delete().root;
            row = style_export::cont_chat_editor_row(style_export::ContChatEditorRowArgs {
                left: vec![delete.clone()],
                middle: file_el.root,
            }).root;
            name = Some(file_el.name);
        }
        delete.ref_on("click", {
            let editor = Rc::downgrade(self);
            let row = row.weak();
            move |ev| {
                ev.stop_propagation();
                let Some(editor) = editor.upgrade() else {
                    return;
                };
                let Some(row) = row.upgrade() else {
                    return;
                };
                editor.files.borrow_mut().retain(|f| f.row.ptr_id() != row.ptr_id());
                editor.render();
            }
        });
        self.files.borrow_mut().push(EditorFile {
            file: file,
            row: row,
            name: name,
        });
    }

    fn render(self: &Rc<Self>) {
        self.rows.ref_clear();
        let close_cell = vec![self.close.clone()];
        let text = self.text.borrow();
        let deleting = self.deleting.get();
        let hidden = &style_export::class_state_hidden().value;
        if self.toolbox_open.get() {
            self.toolbox_text.ref_modify_classes(&[(hidden, text.is_some() || deleting)]);
            self.toolbox_camera.ref_modify_classes(&[(hidden, deleting)]);
            self.toolbox_file.ref_modify_classes(&[(hidden, deleting)]);
            self.toolbox_event.ref_modify_classes(&[(hidden, deleting)]);
            if let Some(toolbox_delete) = &self.toolbox_delete {
                toolbox_delete.ref_modify_classes(&[(&style_export::class_state_selected().value, deleting)]);
            }
            self.rows.ref_push(style_export::cont_chat_editor_row(style_export::ContChatEditorRowArgs {
                left: close_cell.clone(),
                middle: self.toolbox.clone(),
            }).root);
        }
        if deleting {
            return;
        }
        if let Some(text) = &*text {
            text.plus_slot.ref_clear();
            let left;
            if self.toolbox_open.get() {
                let delete = style_export::leaf_chat_editor_button_delete().root;
                delete.ref_on("click", {
                    let editor = Rc::downgrade(self);
                    move |_| {
                        let Some(editor) = editor.upgrade() else {
                            return;
                        };
                        let empty =
                            editor
                                .text
                                .borrow()
                                .as_ref()
                                .map(|t| t.editor.get_text().trim().is_empty())
                                .unwrap_or(true);
                        if empty {
                            *editor.text.borrow_mut() = None;
                            editor.render();
                            return;
                        }
                        let modal =
                            style_export::cont_chat_editor_modal_confirm(
                                style_export::ContChatEditorModalConfirmArgs { text: format!("Delete the text?") },
                            );
                        let handle = ModalHandle::new(&modal.root);
                        modal.cancel.ref_on("click", {
                            let handle = handle.clone();
                            move |_| handle.close()
                        });
                        modal.ok.ref_on("click", {
                            let editor = Rc::downgrade(&editor);
                            let handle = handle.clone();
                            move |_| {
                                handle.close();
                                let Some(editor) = editor.upgrade() else {
                                    return;
                                };
                                *editor.text.borrow_mut() = None;
                                editor.render();
                            }
                        });
                        handle.open();
                    }
                });
                left = vec![delete];
            } else {
                text.plus_slot.ref_push(self.plus.clone());
                left = close_cell.clone();
            }
            self.rows.ref_push(style_export::cont_chat_editor_row(style_export::ContChatEditorRowArgs {
                left: left,
                middle: text.root.clone(),
            }).root);
        }
        for event in &*self.events.borrow() {
            self.rows.ref_push(event.row.clone());
        }
        let files = self.files.borrow();
        let media = files.iter().filter(|f| f.file.is_media()).map(|f| f.row.clone()).collect::<Vec<_>>();
        if !media.is_empty() {
            self.rows.ref_push(style_export::cont_chat_editor_row(style_export::ContChatEditorRowArgs {
                left: vec![],
                middle: style_export::cont_chat_editor_media(
                    style_export::ContChatEditorMediaArgs { children: media },
                ).root,
            }).root);
        }
        for f in files.iter().filter(|f| !f.file.is_media()) {
            self.rows.ref_push(f.row.clone());
        }
        for (_, row) in &*self.extensions.borrow() {
            self.rows.ref_push(row.clone());
        }
    }

    fn focus(&self) {
        let Some(text) = &*self.text.borrow() else {
            return;
        };
        text.editor_el.html().focus().unwrap();
        let range = web_sys::window().unwrap().document().unwrap().create_range().unwrap();
        range.select_node_contents(&text.editor_el.raw()).unwrap();
        range.collapse_with_to_start(false);
        let selection = web_sys::window().unwrap().get_selection().unwrap().unwrap();
        selection.remove_all_ranges().unwrap();
        selection.add_range(&range).unwrap();
    }

    fn blocks(&self) -> Vec<ViewBlock> {
        let mut out = vec![];
        if self.deleting.get() {
            return out;
        }
        if let Some(text) = &*self.text.borrow() {
            let text = text.editor.get_text();
            if !text.trim().is_empty() {
                out.push(ViewBlock::Text(text));
            }
        }
        for f in &*self.files.borrow() {
            let mut file = f.file.clone();
            if let Some(name) = &f.name {
                let name = name.raw().dyn_into::<HtmlInputElement>().unwrap().value().trim().to_string();
                file.name = if name.is_empty() {
                    None
                } else {
                    Some(name)
                };
            }
            out.push(ViewBlock::File(file));
        }
        for event in &*self.events.borrow() {
            out.push(ViewBlock::Event(event.event.borrow().clone()));
        }
        for (extension, _) in &*self.extensions.borrow() {
            out.push(ViewBlock::Extension(extension.clone()));
        }
        return out;
    }
}

fn editor_calendar_channel(channel: &QualifiedChannelId, own_identity: &Identity) -> CalChannel {
    return CalChannel {
        id: channel.clone(),
        own_identity: own_identity.clone(),
    };
}

fn configure_send(
    pc: &mut ProcessingContext,
    editor: &Rc<MessageEditor>,
    mode: &HistPrim<ChatMode>,
    chat_state2: &Rc<ChatState2>,
    channel: QualifiedChannelId,
    own_identity: Identity,
) {
    editor.send.ref_on("click", {
        let editor = Rc::downgrade(editor);
        let mode = mode.clone();
        let eg = pc.eg();
        let chat_state2 = chat_state2.clone();
        move |_| {
            let Some(editor) = editor.upgrade() else {
                return;
            };
            let blocks = editor.blocks();
            if blocks.is_empty() {
                return;
            }
            let client_id = match new_message_client_id() {
                Ok(c) => c,
                Err(e) => {
                    state().log.log(&e);
                    return;
                },
            };
            let reply_to = match &*mode.borrow() {
                ChatMode::ReplyMessage(c) => Some(match &c.target {
                    ChatModeReplyMessageTarget::Channel(m) => OutboxMessageReplyTo::Channel(m.clone()),
                    ChatModeReplyMessageTarget::Outbox(m) => OutboxMessageReplyTo::Outbox(m.message.clone()),
                }),
                _ => None,
            };
            editor.load(&[]);
            editor.focus();
            let outbox_feed =
                chat_state2
                    .channel_lookup
                    .borrow()
                    .get(&channel)
                    .and_then(|f| f.outboxes.borrow().get(&own_identity).cloned());
            if let Some(f) = outbox_feed {
                f.add_sent(client_id.clone(), blocks.clone());
            }
            spawn_log("Saving sent message to outbox", {
                let channel = channel.clone();
                let own_identity = own_identity.clone();
                let client_id = client_id.clone();
                let rel = match &reply_to {
                    None => OutboxMessageRel::None,
                    Some(r) => OutboxMessageRel::ReplyTo(r.clone()),
                };
                async move {
                    return opfs_write_outbox_message(&own_identity, &channel, rel, client_id, blocks).await;
                }
            });
            eg.event(|pc| {
                mode.set(pc, ChatMode::ReplyMessage(ChatModeReplyMessage {
                    target: ChatModeReplyMessageTarget::Outbox(ChatModeReplyMessageTargetOutbox {
                        channel: channel.clone(),
                        message: client_id.clone(),
                    }),
                    own_identity: own_identity.clone(),
                }));
            }).unwrap();
        }
    });
}

pub fn build_chat_entry_controls(pc: &mut ProcessingContext, m: &ChatEntryControls) -> El {
    let out = style_export::cont_chat_entry_mode_controls().root;
    out.ref_own(
        |out| link!(
            (pc = pc),
            (channels = m.state.channels_meta.clone()),
            (),
            (out = out.weak(), mode = m.mode.clone()) {
                let out = out.upgrade()?;
                out.ref_clear();
                if channels.borrow().is_empty() {
                    return None;
                }
                out.ref_push(style_export::leaf_chat_entry_mode_controls_button_new_message().root.on("click", {
                    let eg = pc.eg();
                    let channels = channels.clone();
                    let mode = mode.clone();
                    move |_| eg.event(|pc| {
                        let channels = channels.borrow();
                        if channels.len() > 1 {
                            mode.set(pc, ChatMode::MessageChannelSelect);
                        } else if let Some(channel) = channels.get(0) {
                            mode.set(pc, ChatMode::TopMessage(ChatModeTopMessage {
                                channel: channel.2.clone(),
                                own_identity: channel.1.clone(),
                            }));
                        }
                    }).unwrap()
                }));
            }
        ),
    );
    return out;
}

pub fn build_controls(
    pc: &mut ProcessingContext,
    inf_post: &El,
    chat_state: Rc<ChatState>,
    chat_state2: Rc<ChatState2>,
) {
    inf_post.ref_own(
        |inf_post| link!(
            (pc = pc),
            (mode = chat_state.mode.clone()),
            (),
            (
                chat_state = chat_state,
                chat_state2 = chat_state2,
                inf_post = inf_post.weak(),
                menu_link = RefCell::new(None),
                reply_controls = RefCell::new(None)
            ) {
                let inf_post = inf_post.upgrade()?;
                let new_reply_controls = match &*mode.borrow() {
                    ChatMode::ReplyMessage(c) => Some((c.target.channel().clone(), c.own_identity.clone())),
                    _ => None,
                };
                if new_reply_controls.is_some() && *reply_controls.borrow() == new_reply_controls {
                    return None;
                }
                *reply_controls.borrow_mut() = new_reply_controls;
                *menu_link.borrow_mut() = None;
                inf_post.ref_clear();
                let inf_post_replace;
                let mut focus = None;
                match &*mode.borrow() {
                    ChatMode::None => {
                        inf_post_replace = vec![];
                    },
                    ChatMode::MessageChannelSelect => {
                        *menu_link.borrow_mut() =
                            Some(
                                scope_any(
                                    link!(
                                        (pc = pc),
                                        (channels = chat_state.channels_meta.clone()),
                                        (),
                                        (mode = mode.clone(), inf_post = inf_post.weak()) {
                                            let inf_post = inf_post.upgrade()?;
                                            inf_post.ref_clear();
                                            let mut children = vec![];
                                            for channel in &*channels.borrow() {
                                                children.push(
                                                    style_export::leaf_chat_controls_bar_mode_menu_button_new_message()
                                                        .root
                                                        .on("click", {
                                                            let mode = mode.clone();
                                                            let eg = pc.eg();
                                                            let sender = channel.1.clone();
                                                            let channel = channel.2.clone();
                                                            move |_| eg.event(|pc| {
                                                                mode.set(pc, ChatMode::TopMessage(ChatModeTopMessage {
                                                                    channel: channel.clone(),
                                                                    own_identity: sender.clone(),
                                                                }));
                                                            }).unwrap()
                                                        }),
                                                );
                                            }
                                            inf_post.ref_push(
                                                style_export::cont_chat_controls_bar_mode_menu(
                                                    style_export::ContChatControlsBarModeMenuArgs {
                                                        children: children
                                                    },
                                                ).root,
                                            );
                                        }
                                    ),
                                ),
                            );
                        inf_post_replace = vec![];
                    },
                    ChatMode::TopMessage(c) => {
                        let controls = style_export::leaf_chat_editor_mode_message();
                        let editor =
                            MessageEditor::new(
                                editor_calendar_channel(&c.channel, &c.own_identity),
                                controls.rows,
                                controls.send,
                                false,
                                &[],
                            );
                        editor.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                mode.set(pc, ChatMode::None);
                            }).unwrap()
                        });
                        configure_send(pc, &editor, &mode, &chat_state2, c.channel.clone(), c.own_identity.clone());
                        focus = Some(editor.clone());
                        controls.root.ref_own(|_| editor);
                        inf_post_replace = vec![controls.root];
                    },
                    ChatMode::ReplyMessage(c) => {
                        let controls = style_export::leaf_chat_editor_mode_message();
                        let editor =
                            MessageEditor::new(
                                editor_calendar_channel(c.target.channel(), &c.own_identity),
                                controls.rows,
                                controls.send,
                                false,
                                &[],
                            );
                        editor.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                mode.set(pc, ChatMode::None);
                            }).unwrap()
                        });
                        configure_send(
                            pc,
                            &editor,
                            &mode,
                            &chat_state2,
                            c.target.channel().clone(),
                            c.own_identity.clone(),
                        );
                        focus = Some(editor.clone());
                        controls.root.ref_own(|_| editor);
                        inf_post_replace = vec![controls.root];
                    },
                    ChatMode::EditMessage(c) => {
                        let controls = style_export::leaf_chat_editor_mode_edit();
                        let editor =
                            MessageEditor::new(
                                editor_calendar_channel(&c.target.channel, &c.own_identity),
                                controls.rows,
                                controls.send,
                                true,
                                &c.blocks.0,
                            );
                        editor.close.ref_on("click", {
                            let mode = mode.clone();
                            let eg = pc.eg();
                            let reply_mode = ChatMode::ReplyMessage(ChatModeReplyMessage {
                                target: ChatModeReplyMessageTarget::Channel(c.target.clone()),
                                own_identity: c.own_identity.clone(),
                            });
                            move |_| eg.event(|pc| {
                                mode.set(pc, reply_mode.clone());
                            }).unwrap()
                        });
                        editor.send.ref_on("click", {
                            let editor = Rc::downgrade(&editor);
                            let edit = c.clone();
                            let eg = pc.eg();
                            let chat_state = Rc::downgrade(chat_state);
                            move |_| {
                                let Some(chat_state) = chat_state.upgrade() else {
                                    return;
                                };
                                let Some(editor) = editor.upgrade() else {
                                    return;
                                };
                                let blocks = editor.blocks();
                                if blocks.is_empty() && !editor.deleting.get() {
                                    return;
                                }
                                let reply_mode = ChatMode::ReplyMessage(ChatModeReplyMessage {
                                    target: ChatModeReplyMessageTarget::Channel(edit.target.clone()),
                                    own_identity: edit.own_identity.clone(),
                                });
                                if blocks == **edit.blocks {
                                    eg.event(|pc| {
                                        chat_state.mode.set(pc, reply_mode);
                                    }).unwrap();
                                    return;
                                }
                                let client_id = match new_message_client_id() {
                                    Ok(c) => c,
                                    Err(e) => {
                                        state().log.log(&e);
                                        return;
                                    },
                                };
                                spawn_log("Saving message edit to outbox", {
                                    let channel = edit.target.channel.clone();
                                    let own_identity = edit.own_identity.clone();
                                    let rel = OutboxMessageRel::EditOf(edit.target.clone());
                                    let client_id = client_id.clone();
                                    let blocks = blocks.clone();
                                    async move {
                                        return opfs_write_outbox_message(
                                            &own_identity,
                                            &channel,
                                            rel,
                                            client_id,
                                            blocks
                                        ).await;
                                    }
                                });
                                eg.event(|pc| {
                                    chat_state.apply_edit_overlay(pc, edit.target.clone(), &client_id, blocks);
                                    chat_state.mode.set(pc, reply_mode);
                                }).unwrap();
                            }
                        });
                        focus = Some(editor.clone());
                        controls.root.ref_own(|_| editor);
                        inf_post_replace = vec![controls.root];
                    },
                }
                inf_post.ref_extend(inf_post_replace);
                if let Some(focus) = focus {
                    focus.focus();
                }
            }
        ),
    );
}
