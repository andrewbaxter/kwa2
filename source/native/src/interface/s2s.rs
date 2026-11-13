use {
    glove::reqresp,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::shared::{
        ChannelInviteToken,
        IdentityInviteToken,
    },
    std::net::IpAddr,
};

pub const NOTIFY_MAX_AGE_SECS: i64 = 300;
pub const SPAGH_RECORD_KEY: &str = "kwa";

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SpaghRecordV1 {
    pub ips: Vec<IpAddr>,
    pub port: u16,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SpaghRecord {
    V1(SpaghRecordV1),
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InvitationToken {
    Channel(ChannelInviteToken),
    Identity(IdentityInviteToken),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum S2sErrorKind {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Internal,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct S2sErrorBody {
    pub kind: S2sErrorKind,
    pub message: String,
}

pub use s2sv1t::*;

pub mod s2sv1t {
    use {
        jiff::Timestamp,
        schemars::JsonSchema,
        serde::{
            Deserialize,
            Serialize,
        },
        shared::interface::{
            shared::{
                ChannelId,
                Level,
                Message,
                MessageClientId,
                FileHash,
                FileRef,
                MessageId,
                QualifiedChannelId,
            },
            wire::c2s::{
                calendar_path_parse,
                calendar_path_segments,
                moderation_path_parse,
                moderation_path_segments,
                CalendarMonth,
                CalendarToken,
                ChannelInviteCreate,
                ChannelInviteDelete,
                ChannelInviteModify,
                ChannelInviteRes,
                ChannelMemberRes,
                MemberInfoRes,
                ModerationCursor,
                ModerationEntry,
                confirm_path_const,
                confirm_path_element,
                confirm_path_empty,
                deserialize_path,
                serialize_path,
                PathReqTrait,
            },
        },
        spaghettinuum::{
            byteszb32::BytesZb32,
            interface::identity::{
                Identity,
                IdentityPublicKey,
            },
            jsonsig::JsonSignature,
            sigdomain::{
                SigDomain,
                SigDomained,
            },
        },
    };

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct NotifyTimestamp {
        pub owner: Identity,
        pub channel: ChannelId,
        pub timestamp: Timestamp,
    }

    impl SigDomained for NotifyTimestamp {
        const DOMAIN: SigDomain = SigDomain::App("kwa/notify_timestamp");
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct StartIdentify {
        pub identity: Identity,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct StartIdentifyRes {
        pub challenge: BytesZb32,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentifyChallenge {
        pub challenge: BytesZb32,
        pub audience: Identity,
    }

    impl SigDomained for IdentifyChallenge {
        const DOMAIN: SigDomain = SigDomain::App("kwa/identify_challenge");
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Identify {
        pub identity: Identity,
        pub challenge: JsonSignature<IdentifyChallenge, IdentityPublicKey>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentifyRes {
        pub token: String,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Notify {
        pub signer: Identity,
        pub signed: JsonSignature<NotifyTimestamp, IdentityPublicKey>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum ChannelOrIdentity {
        Channel(ChannelId),
        Identity(Identity),
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Join {
        pub target: ChannelOrIdentity,
        pub invitation: super::InvitationToken,
    }

    #[derive(Serialize, Deserialize, JsonSchema, Clone)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelInfoRes {
        pub page_size: u32,
        pub latest_seq: Option<u64>,
        pub latest_snap_offset: Option<u64>,
        pub earliest_snap_offset: Option<u64>,
        pub level: Level,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct JoinRes {
        pub channel: ChannelId,
        pub info: ChannelInfoRes,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct PullOutbox {
        pub channel: QualifiedChannelId,
        pub host: Identity,
        pub after: Option<u64>,
        pub limit: u32,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct PulledMessage {
        pub seq: u64,
        pub snap_offset: Option<u64>,
        pub receive_time: Timestamp,
        pub message: Message,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct PulledMessages {
        pub messages: Vec<PulledMessage>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct PulledDeleted {
        pub seq: u64,
        pub id: MessageId,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum PulledEntry {
        Message(PulledMessage),
        Deleted(PulledDeleted),
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct PulledActivity {
        pub entries: Vec<PulledEntry>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum ModerateReq {
        Decide {
            message: MessageId,
            approve: bool,
        },
        MemberSet {
            member: Identity,
            level: Level,
        },
        MemberRemove {
            member: Identity,
        },
        InviteCreate(ChannelInviteCreate),
        InviteModify(ChannelInviteModify),
        InviteDelete(ChannelInviteDelete),
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Moderate {
        pub channel: QualifiedChannelId,
        pub req: ModerateReq,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum ModerateRes {
        None,
        Invite(ChannelInviteRes),
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct SnapLookupRes {
        pub original_id: MessageId,
        pub snap_offset: u64,
        pub receive_time: Timestamp,
    }

    pub const GET_PREFIX_CHANNEL_INFO: &str = "channel_info";
    pub const GET_PREFIX_ACTIVITY: &str = "activity";
    pub const GET_PREFIX_SNAP_PAGE: &str = "snap_page";
    pub const GET_PREFIX_SNAP_PAGE_CONTAINING_TIME: &str = "snap_page_containing_time";
    pub const GET_PREFIX_SNAP_LOOKUP: &str = "snap_lookup";
    pub const GET_PREFIX_MESSAGE_HISTORY: &str = "message_history";
    pub const GET_PREFIX_MEMBER_INFO: &str = "member_info";
    pub const GET_PREFIX_MODERATION: &str = "moderation";
    pub const GET_PREFIX_MEMBERS: &str = "members";
    pub const GET_PREFIX_INVITES: &str = "invites";
    pub const GET_PREFIX_CALENDAR_PAGE: &str = "calendar_page";
    pub const GET_PREFIX_JOIN_PREVIEW: &str = "join_preview";
    pub const GET_PREFIX_FILE: &str = "file";
    const FILE_AUTH_KIND_CALLER: &str = "t";
    const FILE_AUTH_KIND_CHANNEL: &str = "m";
    const FILE_VARIANT_ORIGINAL: &str = "o";
    const FILE_VARIANT_THUMBNAIL: &str = "t";
    const JOIN_PREVIEW_KIND_IDENTITY_INVITE: &str = "i";
    const JOIN_PREVIEW_KIND_CHANNEL_INVITE: &str = "c";
    const JOIN_PREVIEW_KIND_CHANNEL: &str = "p";

    pub enum JoinPreviewTarget {
        IdentityInvite {
            identity: Identity,
            code: String,
        },
        ChannelInvite {
            channel: QualifiedChannelId,
            code: String,
        },
        Channel(QualifiedChannelId),
    }

    pub struct GetJoinPreview {
        pub target: JoinPreviewTarget,
    }

    pub enum FileAuth {
        Caller,
        Channel(QualifiedChannelId),
        IdentityInvite {
            identity: Identity,
            code: String,
        },
        ChannelInvite {
            channel: QualifiedChannelId,
            code: String,
        },
    }

    pub struct GetFile {
        pub auth: FileAuth,
        pub hash: FileHash,
        pub thumbnail: bool,
    }

    impl PathReqTrait for GetFile {
        type Resp = Vec<u8>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_FILE)?;
            let variant: String = confirm_path_element(&mut parts, "variant")?;
            let thumbnail = match variant.as_str() {
                FILE_VARIANT_ORIGINAL => false,
                FILE_VARIANT_THUMBNAIL => true,
                _ => return Err(format!("Unknown file variant [{}]", variant)),
            };
            let kind: String = confirm_path_element(&mut parts, "auth kind")?;
            let auth = match kind.as_str() {
                FILE_AUTH_KIND_CALLER => FileAuth::Caller,
                FILE_AUTH_KIND_CHANNEL => FileAuth::Channel(channel_from_path(&mut parts)?),
                JOIN_PREVIEW_KIND_IDENTITY_INVITE => FileAuth::IdentityInvite {
                    identity: confirm_path_element(&mut parts, "identity")?,
                    code: confirm_path_element(&mut parts, "code")?,
                },
                JOIN_PREVIEW_KIND_CHANNEL_INVITE => FileAuth::ChannelInvite {
                    channel: channel_from_path(&mut parts)?,
                    code: confirm_path_element(&mut parts, "code")?,
                },
                _ => return Err(format!("Unknown file auth kind [{}]", kind)),
            };
            let hash = confirm_path_element(&mut parts, "hash")?;
            confirm_path_empty(&mut parts)?;
            return Ok(GetFile {
                auth: auth,
                hash: hash,
                thumbnail: thumbnail,
            });
        }

        fn serialize_path(&self) -> String {
            let auth: Vec<String> = match &self.auth {
                FileAuth::Caller => vec![FILE_AUTH_KIND_CALLER.to_string()],
                FileAuth::Channel(channel) => {
                    [FILE_AUTH_KIND_CHANNEL.to_string()].into_iter().chain(channel_segments(channel)).collect()
                },
                FileAuth::IdentityInvite { identity, code } => vec![
                    JOIN_PREVIEW_KIND_IDENTITY_INVITE.to_string(),
                    identity.to_string(),
                    code.clone(),
                ],
                FileAuth::ChannelInvite { channel, code } => {
                    [JOIN_PREVIEW_KIND_CHANNEL_INVITE.to_string()]
                        .into_iter()
                        .chain(channel_segments(channel))
                        .chain([code.clone()])
                        .collect()
                },
            };
            return serialize_path([GET_PREFIX_FILE.to_string(), if self.thumbnail {
                FILE_VARIANT_THUMBNAIL
            } else {
                FILE_VARIANT_ORIGINAL
            }.to_string()].into_iter().chain(auth).chain([self.hash.to_string()]));
        }
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct JoinPreviewRes {
        pub name: String,
        pub description: String,
        pub portrait: Option<FileRef>,
    }

    impl PathReqTrait for GetJoinPreview {
        type Resp = JoinPreviewRes;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_JOIN_PREVIEW)?;
            let kind: String = confirm_path_element(&mut parts, "preview kind")?;
            let target = match kind.as_str() {
                JOIN_PREVIEW_KIND_IDENTITY_INVITE => JoinPreviewTarget::IdentityInvite {
                    identity: confirm_path_element(&mut parts, "identity")?,
                    code: confirm_path_element(&mut parts, "code")?,
                },
                JOIN_PREVIEW_KIND_CHANNEL_INVITE => JoinPreviewTarget::ChannelInvite {
                    channel: channel_from_path(&mut parts)?,
                    code: confirm_path_element(&mut parts, "code")?,
                },
                JOIN_PREVIEW_KIND_CHANNEL => JoinPreviewTarget::Channel(channel_from_path(&mut parts)?),
                _ => return Err(format!("Unknown preview kind [{}]", kind)),
            };
            confirm_path_empty(&mut parts)?;
            return Ok(GetJoinPreview { target: target });
        }

        fn serialize_path(&self) -> String {
            let tail: Vec<String> = match &self.target {
                JoinPreviewTarget::IdentityInvite { identity, code } => vec![
                    JOIN_PREVIEW_KIND_IDENTITY_INVITE.to_string(),
                    identity.to_string(),
                    code.clone(),
                ],
                JoinPreviewTarget::ChannelInvite { channel, code } => {
                    [JOIN_PREVIEW_KIND_CHANNEL_INVITE.to_string()]
                        .into_iter()
                        .chain(channel_segments(channel))
                        .chain([code.clone()])
                        .collect()
                },
                JoinPreviewTarget::Channel(channel) => {
                    [JOIN_PREVIEW_KIND_CHANNEL.to_string()].into_iter().chain(channel_segments(channel)).collect()
                },
            };
            return serialize_path([GET_PREFIX_JOIN_PREVIEW.to_string()].into_iter().chain(tail));
        }
    }

    const SNAP_LOOKUP_ID: &str = "id";
    const SNAP_LOOKUP_CLIENT_ID: &str = "client_id";

    fn channel_segments(channel: &QualifiedChannelId) -> [String; 2] {
        return [channel.identity.to_string(), channel.channel.0.to_string()];
    }

    fn channel_from_path<
        'a,
    >(parts: &mut dyn Iterator<Item = std::borrow::Cow<'a, str>>) -> Result<QualifiedChannelId, String> {
        return Ok(QualifiedChannelId {
            identity: confirm_path_element(parts, "channel identity")?,
            channel: ChannelId(confirm_path_element(parts, "channel")?),
        });
    }

    pub struct GetChannelInfo {
        pub channel: QualifiedChannelId,
    }

    impl PathReqTrait for GetChannelInfo {
        type Resp = ChannelInfoRes;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_CHANNEL_INFO)?;
            let out = GetChannelInfo { channel: channel_from_path(&mut parts)? };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_CHANNEL_INFO.to_string()].into_iter().chain(channel_segments(&self.channel)),
            );
        }
    }

    pub struct GetActivity {
        pub channel: QualifiedChannelId,
        pub start: u64,
        pub limit: u32,
    }

    impl PathReqTrait for GetActivity {
        type Resp = PulledActivity;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_ACTIVITY)?;
            let out = GetActivity {
                channel: channel_from_path(&mut parts)?,
                start: confirm_path_element(&mut parts, "start")?,
                limit: confirm_path_element(&mut parts, "limit")?,
            };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_ACTIVITY.to_string()]
                    .into_iter()
                    .chain(channel_segments(&self.channel))
                    .chain([self.start.to_string(), self.limit.to_string()]),
            );
        }
    }

    pub struct GetSnapPage {
        pub channel: QualifiedChannelId,
        pub page: u64,
    }

    impl PathReqTrait for GetSnapPage {
        type Resp = Option<PulledMessages>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_SNAP_PAGE)?;
            let out = GetSnapPage {
                channel: channel_from_path(&mut parts)?,
                page: confirm_path_element(&mut parts, "page")?,
            };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_SNAP_PAGE.to_string()]
                    .into_iter()
                    .chain(channel_segments(&self.channel))
                    .chain([self.page.to_string()]),
            );
        }
    }

    pub struct GetSnapPageContainingTime {
        pub channel: QualifiedChannelId,
        pub time: Timestamp,
    }

    impl PathReqTrait for GetSnapPageContainingTime {
        type Resp = Option<u64>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_SNAP_PAGE_CONTAINING_TIME)?;
            let out = GetSnapPageContainingTime {
                channel: channel_from_path(&mut parts)?,
                time: confirm_path_element(&mut parts, "time")?,
            };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_SNAP_PAGE_CONTAINING_TIME.to_string()]
                    .into_iter()
                    .chain(channel_segments(&self.channel))
                    .chain([self.time.to_string()]),
            );
        }
    }

    pub enum SnapLookupBy {
        Id(MessageId),
        ClientId(Identity, MessageClientId),
    }

    pub struct GetSnapLookup {
        pub channel: QualifiedChannelId,
        pub by: SnapLookupBy,
    }

    impl PathReqTrait for GetSnapLookup {
        type Resp = Option<SnapLookupRes>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_SNAP_LOOKUP)?;
            let channel = channel_from_path(&mut parts)?;
            let kind: String = confirm_path_element(&mut parts, "lookup kind")?;
            let by = match kind.as_str() {
                SNAP_LOOKUP_ID => SnapLookupBy::Id(MessageId {
                    identity: confirm_path_element(&mut parts, "message sender")?,
                    unique: confirm_path_element(&mut parts, "message unique")?,
                }),
                SNAP_LOOKUP_CLIENT_ID => SnapLookupBy::ClientId(
                    confirm_path_element(&mut parts, "message sender")?,
                    MessageClientId(confirm_path_element(&mut parts, "message client id")?),
                ),
                other => return Err(format!("Unknown snap lookup kind [{}]", other)),
            };
            confirm_path_empty(&mut parts)?;
            return Ok(GetSnapLookup {
                channel: channel,
                by: by,
            });
        }

        fn serialize_path(&self) -> String {
            let tail = match &self.by {
                SnapLookupBy::Id(id) => vec![
                    SNAP_LOOKUP_ID.to_string(),
                    id.identity.to_string(),
                    id.unique.to_string(),
                ],
                SnapLookupBy::ClientId(sender, client_id) => vec![
                    SNAP_LOOKUP_CLIENT_ID.to_string(),
                    sender.to_string(),
                    client_id.0.clone(),
                ],
            };
            return serialize_path(
                [GET_PREFIX_SNAP_LOOKUP.to_string()].into_iter().chain(channel_segments(&self.channel)).chain(tail),
            );
        }
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct CalendarPulledPage {
        pub token: CalendarToken,
        pub latest_seq: Option<u64>,
        pub messages: Vec<PulledMessage>,
        pub last: bool,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum CalendarPulledRes {
        Page(CalendarPulledPage),
        Stale,
    }

    pub struct GetCalendarPage {
        pub channel: QualifiedChannelId,
        pub month: CalendarMonth,
        pub page: usize,
        pub token: Option<CalendarToken>,
    }

    impl PathReqTrait for GetCalendarPage {
        type Resp = Option<CalendarPulledRes>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_CALENDAR_PAGE)?;
            let (channel, month, page, token) = calendar_path_parse(&mut parts)?;
            confirm_path_empty(&mut parts)?;
            return Ok(GetCalendarPage {
                channel: channel,
                month: month,
                page: page,
                token: token,
            });
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_CALENDAR_PAGE.to_string()]
                    .into_iter()
                    .chain(calendar_path_segments(&self.channel, self.month, self.page, self.token)),
            );
        }
    }

    pub struct GetMemberInfo {
        pub channel: QualifiedChannelId,
        pub member: Identity,
    }

    impl PathReqTrait for GetMemberInfo {
        type Resp = Option<MemberInfoRes>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_MEMBER_INFO)?;
            let out = GetMemberInfo {
                channel: channel_from_path(&mut parts)?,
                member: confirm_path_element(&mut parts, "member")?,
            };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_MEMBER_INFO.to_string()]
                    .into_iter()
                    .chain(channel_segments(&self.channel))
                    .chain([self.member.to_string()]),
            );
        }
    }

    pub struct GetModeration {
        pub channel: QualifiedChannelId,
        pub cursor: ModerationCursor,
    }

    impl PathReqTrait for GetModeration {
        type Resp = Vec<ModerationEntry>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_MODERATION)?;
            let (channel, cursor) = moderation_path_parse(&mut parts)?;
            confirm_path_empty(&mut parts)?;
            return Ok(GetModeration {
                channel: channel,
                cursor: cursor,
            });
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_MODERATION.to_string()]
                    .into_iter()
                    .chain(moderation_path_segments(&self.channel, self.cursor)),
            );
        }
    }

    pub struct GetMembers {
        pub channel: QualifiedChannelId,
    }

    impl PathReqTrait for GetMembers {
        type Resp = Vec<ChannelMemberRes>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_MEMBERS)?;
            let out = GetMembers { channel: channel_from_path(&mut parts)? };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_MEMBERS.to_string()].into_iter().chain(channel_segments(&self.channel)),
            );
        }
    }

    pub struct GetInvites {
        pub channel: QualifiedChannelId,
    }

    impl PathReqTrait for GetInvites {
        type Resp = Vec<ChannelInviteRes>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_INVITES)?;
            let out = GetInvites { channel: channel_from_path(&mut parts)? };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_INVITES.to_string()].into_iter().chain(channel_segments(&self.channel)),
            );
        }
    }

    pub struct GetMessageHistory {
        pub channel: QualifiedChannelId,
        pub id: MessageId,
    }

    impl PathReqTrait for GetMessageHistory {
        type Resp = Option<PulledMessages>;

        fn deserialize_path(path: &str) -> Result<Self, String> {
            let mut parts = deserialize_path(path);
            confirm_path_const(&mut parts, GET_PREFIX_MESSAGE_HISTORY)?;
            let out = GetMessageHistory {
                channel: channel_from_path(&mut parts)?,
                id: MessageId {
                    identity: confirm_path_element(&mut parts, "message sender")?,
                    unique: confirm_path_element(&mut parts, "message unique")?,
                },
            };
            confirm_path_empty(&mut parts)?;
            return Ok(out);
        }

        fn serialize_path(&self) -> String {
            return serialize_path(
                [GET_PREFIX_MESSAGE_HISTORY.to_string()]
                    .into_iter()
                    .chain(channel_segments(&self.channel))
                    .chain([self.id.identity.to_string(), self.id.unique.to_string()]),
            );
        }
    }
}

reqresp!(pub s2sv1 {
    StartIdentify(s2sv1t::StartIdentify) => s2sv1t:: StartIdentifyRes,
    Identify(s2sv1t::Identify) => s2sv1t:: IdentifyRes,
    Notify(s2sv1t::Notify) =>(),
    Join(s2sv1t::Join) => s2sv1t:: JoinRes,
    PullOutbox(s2sv1t::PullOutbox) => s2sv1t:: PulledMessages,
    Moderate(s2sv1t::Moderate) => s2sv1t:: ModerateRes,
});
