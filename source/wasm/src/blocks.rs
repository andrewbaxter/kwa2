use {
    crate::state::state,
    shared::interface::{
        shared::{
            mime_is_media,
            MessageBlock,
            MessageBlockEvent,
            MessageBlockExtension,
            QualifiedChannelId,
            QualifiedFileId,
        },
        wire::c2s::{
            GetChannelFile,
            PathReqTrait,
        },
    },
    std::rc::Rc,
    web_sys::{
        Blob,
        Url,
    },
};

pub struct LocalBlob {
    pub blob: Blob,
    pub url: String,
}

impl LocalBlob {
    pub fn new(blob: Blob) -> Rc<LocalBlob> {
        let url = Url::create_object_url_with_blob(&blob).unwrap();
        return Rc::new(LocalBlob {
            blob: blob,
            url: url,
        });
    }
}

impl Drop for LocalBlob {
    fn drop(&mut self) {
        Url::revoke_object_url(&self.url).unwrap();
    }
}

#[derive(Clone)]
pub enum ViewFileSource {
    Remote {
        channel: QualifiedChannelId,
        file: QualifiedFileId,
    },
    Local(Rc<LocalBlob>),
}

impl PartialEq for ViewFileSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                ViewFileSource::Remote { channel: a_channel, file: a_file },
                ViewFileSource::Remote { channel: b_channel, file: b_file },
            ) => return a_channel ==
                b_channel &&
                a_file == b_file,
            (ViewFileSource::Local(a), ViewFileSource::Local(b)) => return Rc::ptr_eq(a, b),
            _ => return false,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ViewFile {
    pub source: ViewFileSource,
    pub mime_type: String,
    pub size: u64,
    pub name: Option<String>,
}

impl ViewFile {
    pub fn is_media(&self) -> bool {
        return mime_is_media(&self.mime_type);
    }

    pub fn is_video(&self) -> bool {
        return self.mime_type.starts_with("video/");
    }

    pub fn thumbnail_is_video(&self) -> bool {
        return self.is_video() && matches!(self.source, ViewFileSource::Local(_));
    }

    pub fn url(&self, thumbnail: bool) -> String {
        match &self.source {
            ViewFileSource::Remote { channel, file } => return format!("{}{}", state().env.base_url, GetChannelFile {
                channel: channel.clone(),
                file: file.clone(),
                thumbnail: thumbnail,
            }.serialize_path()),
            ViewFileSource::Local(l) => return l.url.clone(),
        }
    }

    pub fn download_name(&self) -> String {
        let stem = self.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| "file".to_string());
        match mime2ext::mime2ext(self.mime_type.split(';').next().unwrap().trim()) {
            Some(ext) => return format!("{}.{}", stem, ext),
            None => return stem,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum ViewBlock {
    Text(String),
    File(ViewFile),
    Event(MessageBlockEvent),
    Extension(MessageBlockExtension),
}

pub fn view_blocks(channel: &QualifiedChannelId, blocks: Vec<MessageBlock>) -> Vec<ViewBlock> {
    return blocks.into_iter().map(|b| match b {
        MessageBlock::Text(t) => ViewBlock::Text(t),
        MessageBlock::File(f) => ViewBlock::File(ViewFile {
            source: ViewFileSource::Remote {
                channel: channel.clone(),
                file: f.file,
            },
            mime_type: f.mime_type,
            size: f.size,
            name: f.name,
        }),
        MessageBlock::Event(e) => ViewBlock::Event(e),
        MessageBlock::Extension(e) => ViewBlock::Extension(e),
    }).collect();
}

pub struct OrderedBlocks {
    pub texts: Vec<String>,
    pub media: Vec<ViewFile>,
    pub files: Vec<ViewFile>,
    pub events: Vec<MessageBlockEvent>,
    pub extensions: Vec<MessageBlockExtension>,
}

pub fn order_blocks(blocks: &[ViewBlock]) -> OrderedBlocks {
    let mut out = OrderedBlocks {
        texts: vec![],
        media: vec![],
        files: vec![],
        events: vec![],
        extensions: vec![],
    };
    for block in blocks {
        match block {
            ViewBlock::Text(t) => out.texts.push(t.clone()),
            ViewBlock::File(f) => {
                if f.is_media() {
                    out.media.push(f.clone());
                } else {
                    out.files.push(f.clone());
                }
            },
            ViewBlock::Event(e) => out.events.push(e.clone()),
            ViewBlock::Extension(e) => out.extensions.push(e.clone()),
        }
    }
    return out;
}

pub fn format_size(size: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1000. && unit + 1 < units.len() {
        value /= 1000.;
        unit += 1;
    }
    if unit == 0 {
        return format!("{} {}", size, units[unit]);
    }
    return format!("{:.1} {}", value, units[unit]);
}
