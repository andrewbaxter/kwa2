use good_ormning::sqlite::{
    query::expr::SerialExpr,
    schema::field::{
        FieldType,
        field_bool,
        field_i64,
        field_str,
    },
    types::{
        type_i64,
        type_str,
    },
    GenerateArgs,
    Version,
};

fn field_type_opt(mut t: FieldType) -> FieldType {
    t.type_.opt = true;
    return t;
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let version_0;
    {
        let v = Version::new();
        let account_id_t =
            v
                .custom_type("account_id_t")
                .rust_type("crate::interface::db::DbAccountId")
                .base_type(type_i64().build());
        let identity_id_t =
            v
                .custom_type("identity_id_t")
                .rust_type("crate::interface::db::DbIdentity")
                .base_type(type_str().build());
        let identity_secret_t =
            v
                .custom_type("identity_secret_t")
                .rust_type("crate::interface::db::DbIdentitySecret")
                .base_type(type_str().build());
        let refresh_tls_state_t =
            v
                .custom_type("refresh_tls_state_t")
                .rust_type("spaghettinuum_native::interface::stored::self_tls::RefreshTlsState")
                .base_type(type_str().opt().build());
        let channelgroup_id_t =
            v
                .custom_type("channelgroup_id_t")
                .rust_type("crate::interface::db::DbChannelGroupId")
                .base_type(type_i64().build());
        let channel_id_t =
            v
                .custom_type("channel_id_t")
                .rust_type("crate::interface::db::DbChannelId")
                .base_type(type_str().build());
        let time_ms_t =
            v.custom_type("time_ms_t").rust_type("crate::interface::db::DbTimeMs").base_type(type_i64().build());
        {
            let t = v.table("account");
            let _rowid = t.rowid_field(None);
            let id = t.field("external_id", field_str().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            t.unique_index("account_external_id", &[&id]);
        }
        {
            let t = v.table("identity");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            let _secret = t.field("secret", identity_secret_t.field_type());
            let _tls_state = t.field("tls_state", refresh_tls_state_t.field_type());
            t.primary_key("identity_pk", &[&account_id, &id]);
            t.unique_index("identity_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channelgroup");
            let account_id = t.field("account_id", account_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            t.unique_index("channelgroup_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channel");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let id = t.field("id", channel_id_t.field_type());
            let _local_identity = t.field("local_identity", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let mut channel_group_ft = channelgroup_id_t.field_type();
            channel_group_ft.type_.opt = true;
            let _channel_group = t.field("channel_group", channel_group_ft);
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _page_size = t.field("page_size", field_i64().build());
            let _authenticated = t.field("authenticated", field_bool().build());
            let _public_read = t.field("public_read", field_bool().build());
            let _public_post = t.field("public_post", field_bool().build());
            let _retention_secs = t.field("retention_secs", field_i64().opt().build());
            let _backfill_done = t.field("backfill_done", field_bool().build());
            let _latest_seq = t.field("latest_seq", field_i64().opt().build());
            let _latest_snap_offset = t.field("latest_snap_offset", field_i64().opt().build());
            let _floor_snap_offset = t.field("floor_snap_offset", field_i64().opt().build());
            let _next_sender_unique = t.field("next_sender_unique", field_i64().build());
            let _deleted = t.field("deleted", field_type_opt(time_ms_t.field_type()));
            t.primary_key("channel_pk", &[&account_id, &identity, &id]);
            t.unique_index("channel_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_member");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let member = t.field("member", identity_id_t.field_type());
            let _confirmed = t.field("confirmed", field_bool().build());
            let _can_post = t.field("can_post", field_bool().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            t.primary_key("channel_member_pk", &[&account_id, &identity, &channel, &member]);
        }
        {
            let t = v.table("identity_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            t.unique_index("identity_invite_token", &[&identity, &token]);
            t.unique_index("identity_invite_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            let _can_post = t.field("can_post", field_bool().build());
            t.unique_index("channel_invite_token", &[&identity, &channel, &token]);
            t.unique_index("channel_invite_channel_idem", &[&account_id, &identity, &channel, &idem]);
        }
        {
            let t = v.table("message");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let seq = t.field("seq", field_i64().build());
            let snap_offset = t.field("snap_offset", field_i64().opt().build());
            let _receive_time = t.field("receive_time", time_ms_t.field_type());
            let client_id = t.field("client_id", field_str().opt().build());
            let _raw = t.field("raw", field_str().build());
            let _body_text = t.field("body_text", field_str().build());
            let _rel_type = t.field("rel_type", field_str().opt().build());
            let mut rel_target_sender_ft = identity_id_t.field_type();
            rel_target_sender_ft.type_.opt = true;
            let _rel_target_sender = t.field("rel_target_sender", rel_target_sender_ft);
            let _rel_target_unique = t.field("rel_target_unique", field_i64().opt().build());
            t.primary_key("message_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("message_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
            t.unique_index("message_seq", &[&account_id, &identity, &channel, &seq]);
            t.unique_index("message_snap_offset", &[&account_id, &identity, &channel, &snap_offset]);
        }
        {
            let t = v.table("outbox");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let client_id = t.field("client_id", field_str().build());
            let _raw = t.field("raw", field_str().build());
            let _created = t.field("created", time_ms_t.field_type());
            t.primary_key("outbox_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("outbox_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
        }
        {
            let t = v.table("server_settings");
            let key = t.field("key", field_str().build());
            let _value = t.field("value", field_str().build());
            t.primary_key("server_settings_pk", &[&key]);
        }
        {
            let t = v.table("contact");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            t.primary_key("contact_pk", &[&account_id, &id]);
        }
        {
            let t = v.table("file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            t.primary_key("file_pk", &[&account_id, &hash]);
        }
        {
            let t = v.table("push_subscription");
            let account_id = t.field("account_id", account_id_t.field_type());
            let endpoint = t.field("endpoint", field_str().build());
            let _data = t.field("data", field_str().build());
            t.primary_key("push_subscription_pk", &[&account_id, &endpoint]);
        }
        {
            let t = v.table("channel_pull_cursor");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", identity_id_t.field_type());
            let _cursor_seq = t.field("cursor_seq", field_i64().build());
            t.primary_key("channel_pull_cursor_pk", &[&account_id, &identity, &channel, &source]);
        }
        {
            let t = v.table("session");
            let id = t.field("id", field_str().build());
            let _external_id = t.field("external_id", field_str().build());
            let _last_used = t.field("last_used", time_ms_t.field_type());
            t.primary_key("session_pk", &[&id]);
        }
        version_0 = v.build();
    }
    let version_1;
    {
        let v = Version::new();
        let account_id_t =
            v
                .custom_type("account_id_t")
                .rust_type("crate::interface::db::DbAccountId")
                .base_type(type_i64().build());
        let identity_id_t =
            v
                .custom_type("identity_id_t")
                .rust_type("crate::interface::db::DbIdentity")
                .base_type(type_str().build());
        let identity_secret_t =
            v
                .custom_type("identity_secret_t")
                .rust_type("crate::interface::db::DbIdentitySecret")
                .base_type(type_str().build());
        let refresh_tls_state_t =
            v
                .custom_type("refresh_tls_state_t")
                .rust_type("spaghettinuum_native::interface::stored::self_tls::RefreshTlsState")
                .base_type(type_str().opt().build());
        let channelgroup_id_t =
            v
                .custom_type("channelgroup_id_t")
                .rust_type("crate::interface::db::DbChannelGroupId")
                .base_type(type_i64().build());
        let channel_id_t =
            v
                .custom_type("channel_id_t")
                .rust_type("crate::interface::db::DbChannelId")
                .base_type(type_str().build());
        let time_ms_t =
            v.custom_type("time_ms_t").rust_type("crate::interface::db::DbTimeMs").base_type(type_i64().build());
        {
            let t = v.table("account");
            let _rowid = t.rowid_field(None);
            let id = t.field("external_id", field_str().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            t.unique_index("account_external_id", &[&id]);
        }
        {
            let t = v.table("identity");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            let _secret = t.field("secret", identity_secret_t.field_type());
            let _tls_state = t.field("tls_state", refresh_tls_state_t.field_type());
            t.primary_key("identity_pk", &[&account_id, &id]);
            t.unique_index("identity_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channelgroup");
            let account_id = t.field("account_id", account_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            t.unique_index("channelgroup_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channel");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let id = t.field("id", channel_id_t.field_type());
            let _local_identity = t.field("local_identity", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let mut channel_group_ft = channelgroup_id_t.field_type();
            channel_group_ft.type_.opt = true;
            let _channel_group = t.field("channel_group", channel_group_ft);
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _page_size = t.field("page_size", field_i64().build());
            let _authenticated = t.field("authenticated", field_bool().build());
            let _public_read = t.field("public_read", field_bool().build());
            let _public_post = t.field("public_post", field_bool().build());
            let _retention_secs = t.field("retention_secs", field_i64().opt().build());
            let _backfill_done = t.field("backfill_done", field_bool().build());
            let _latest_seq = t.field("latest_seq", field_i64().opt().build());
            let _latest_snap_offset = t.field("latest_snap_offset", field_i64().opt().build());
            let _floor_snap_offset = t.field("floor_snap_offset", field_i64().opt().build());
            let _next_sender_unique = t.field("next_sender_unique", field_i64().build());
            let _deleted = t.field("deleted", field_type_opt(time_ms_t.field_type()));
            t.primary_key("channel_pk", &[&account_id, &identity, &id]);
            t.unique_index("channel_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_member");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let member = t.field("member", identity_id_t.field_type());
            let _confirmed = t.field("confirmed", field_bool().build());
            let _can_post = t.field("can_post", field_bool().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            t.primary_key("channel_member_pk", &[&account_id, &identity, &channel, &member]);
        }
        {
            let t = v.table("identity_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build()).renamed_from("memo_short");
            let _description = t.field("description", field_str().build()).renamed_from("memo_long");
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            t.unique_index("identity_invite_token", &[&identity, &token]);
            t.unique_index("identity_invite_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build()).renamed_from("memo_short");
            let _description = t.field("description", field_str().build()).renamed_from("memo_long");
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            let _can_post = t.field("can_post", field_bool().build());
            t.unique_index("channel_invite_token", &[&identity, &channel, &token]);
            t.unique_index("channel_invite_channel_idem", &[&account_id, &identity, &channel, &idem]);
        }
        {
            let t = v.table("message");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let seq = t.field("seq", field_i64().build());
            let snap_offset = t.field("snap_offset", field_i64().opt().build());
            let _receive_time = t.field("receive_time", time_ms_t.field_type());
            let client_id = t.field("client_id", field_str().opt().build());
            let _raw = t.field("raw", field_str().build());
            let _body_text = t.field("body_text", field_str().build());
            let _rel_type = t.field("rel_type", field_str().opt().build());
            let mut rel_target_sender_ft = identity_id_t.field_type();
            rel_target_sender_ft.type_.opt = true;
            let _rel_target_sender = t.field("rel_target_sender", rel_target_sender_ft);
            let _rel_target_unique = t.field("rel_target_unique", field_i64().opt().build());
            t.primary_key("message_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("message_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
            t.unique_index("message_seq", &[&account_id, &identity, &channel, &seq]);
            t.unique_index("message_snap_offset", &[&account_id, &identity, &channel, &snap_offset]);
        }
        {
            let t = v.table("outbox");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let client_id = t.field("client_id", field_str().build());
            let _raw = t.field("raw", field_str().build());
            let _created = t.field("created", time_ms_t.field_type());
            t.primary_key("outbox_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("outbox_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
        }
        {
            let t = v.table("server_settings");
            let key = t.field("key", field_str().build());
            let _value = t.field("value", field_str().build());
            t.primary_key("server_settings_pk", &[&key]);
        }
        {
            let t = v.table("contact");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            t.primary_key("contact_pk", &[&account_id, &id]);
        }
        {
            let t = v.table("file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            t.primary_key("file_pk", &[&account_id, &hash]);
        }
        {
            let t = v.table("push_subscription");
            let account_id = t.field("account_id", account_id_t.field_type());
            let endpoint = t.field("endpoint", field_str().build());
            let _data = t.field("data", field_str().build());
            t.primary_key("push_subscription_pk", &[&account_id, &endpoint]);
        }
        {
            let t = v.table("channel_pull_cursor");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", identity_id_t.field_type());
            let _cursor_seq = t.field("cursor_seq", field_i64().build());
            t.primary_key("channel_pull_cursor_pk", &[&account_id, &identity, &channel, &source]);
        }
        {
            let t = v.table("session");
            let id = t.field("id", field_str().build());
            let _external_id = t.field("external_id", field_str().build());
            let _last_used = t.field("last_used", time_ms_t.field_type());
            t.primary_key("session_pk", &[&id]);
        }
        version_1 = v.build();
    }
    let version_2;
    {
        let v = Version::new();
        let account_id_t =
            v
                .custom_type("account_id_t")
                .rust_type("crate::interface::db::DbAccountId")
                .base_type(type_i64().build());
        let identity_id_t =
            v
                .custom_type("identity_id_t")
                .rust_type("crate::interface::db::DbIdentity")
                .base_type(type_str().build());
        let identity_secret_t =
            v
                .custom_type("identity_secret_t")
                .rust_type("crate::interface::db::DbIdentitySecret")
                .base_type(type_str().build());
        let refresh_tls_state_t =
            v
                .custom_type("refresh_tls_state_t")
                .rust_type("spaghettinuum_native::interface::stored::self_tls::RefreshTlsState")
                .base_type(type_str().opt().build());
        let channelgroup_id_t =
            v
                .custom_type("channelgroup_id_t")
                .rust_type("crate::interface::db::DbChannelGroupId")
                .base_type(type_i64().build());
        let channel_id_t =
            v
                .custom_type("channel_id_t")
                .rust_type("crate::interface::db::DbChannelId")
                .base_type(type_str().build());
        let time_ms_t =
            v.custom_type("time_ms_t").rust_type("crate::interface::db::DbTimeMs").base_type(type_i64().build());
        {
            let t = v.table("account");
            let _rowid = t.rowid_field(None);
            let id = t.field("external_id", field_str().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            t.unique_index("account_external_id", &[&id]);
        }
        {
            let t = v.table("identity");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            let _secret = t.field("secret", identity_secret_t.field_type());
            let _tls_state = t.field("tls_state", refresh_tls_state_t.field_type());
            t.primary_key("identity_pk", &[&account_id, &id]);
            t.unique_index("identity_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channelgroup");
            let account_id = t.field("account_id", account_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            t.unique_index("channelgroup_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channel");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let id = t.field("id", channel_id_t.field_type());
            let _local_identity = t.field("local_identity", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let mut channel_group_ft = channelgroup_id_t.field_type();
            channel_group_ft.type_.opt = true;
            let _channel_group = t.field("channel_group", channel_group_ft);
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _page_size = t.field("page_size", field_i64().build());
            let _authenticated = t.field("authenticated", field_bool().build());
            let _public_read = t.field("public_read", field_bool().build());
            let _public_post = t.field("public_post", field_bool().build());
            let _retention_secs = t.field("retention_secs", field_i64().opt().build());
            let _backfill_done = t.field("backfill_done", field_bool().build());
            let _latest_seq = t.field("latest_seq", field_i64().opt().build());
            let _latest_snap_offset = t.field("latest_snap_offset", field_i64().opt().build());
            let _floor_snap_offset = t.field("floor_snap_offset", field_i64().opt().build());
            let _next_sender_unique = t.field("next_sender_unique", field_i64().build());
            let _deleted = t.field("deleted", field_type_opt(time_ms_t.field_type()));
            t.primary_key("channel_pk", &[&account_id, &identity, &id]);
            t.unique_index("channel_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_member");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let member = t.field("member", identity_id_t.field_type());
            let _confirmed = t.field("confirmed", field_bool().build());
            let _can_post = t.field("can_post", field_bool().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            t.primary_key("channel_member_pk", &[&account_id, &identity, &channel, &member]);
        }
        {
            let t = v.table("identity_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            t.unique_index("identity_invite_token", &[&identity, &token]);
            t.unique_index("identity_invite_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            let _can_post = t.field("can_post", field_bool().build());
            t.unique_index("channel_invite_token", &[&identity, &channel, &token]);
            t.unique_index("channel_invite_channel_idem", &[&account_id, &identity, &channel, &idem]);
        }
        {
            let t = v.table("message");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let seq = t.field("seq", field_i64().build());
            let snap_offset = t.field("snap_offset", field_i64().opt().build());
            let _receive_time = t.field("receive_time", time_ms_t.field_type());
            let client_id = t.field("client_id", field_str().opt().build());
            let _raw = t.field("raw", field_str().build());
            let _body_text = t.field("body_text", field_str().build());
            let _rel_type = t.field("rel_type", field_str().opt().build());
            let mut rel_target_sender_ft = identity_id_t.field_type();
            rel_target_sender_ft.type_.opt = true;
            let _rel_target_sender = t.field("rel_target_sender", rel_target_sender_ft);
            let _rel_target_unique = t.field("rel_target_unique", field_i64().opt().build());
            t.primary_key("message_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("message_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
            t.unique_index("message_seq", &[&account_id, &identity, &channel, &seq]);
            t.unique_index("message_snap_offset", &[&account_id, &identity, &channel, &snap_offset]);
        }
        {
            let t = v.table("outbox");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let client_id = t.field("client_id", field_str().build());
            let _raw = t.field("raw", field_str().build());
            let _created = t.field("created", time_ms_t.field_type());
            t.primary_key("outbox_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("outbox_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
        }
        {
            let t = v.table("server_settings");
            let key = t.field("key", field_str().build());
            let _value = t.field("value", field_str().build());
            t.primary_key("server_settings_pk", &[&key]);
        }
        {
            let t = v.table("contact");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            t.primary_key("contact_pk", &[&account_id, &id]);
        }
        {
            let t = v.table("file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            t.primary_key("file_pk", &[&account_id, &hash]);
        }
        {
            let t = v.table("push_subscription");
            let account_id = t.field("account_id", account_id_t.field_type());
            let endpoint = t.field("endpoint", field_str().build());
            let _data = t.field("data", field_str().build());
            t.primary_key("push_subscription_pk", &[&account_id, &endpoint]);
        }
        {
            let t = v.table("channel_pull_cursor");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", identity_id_t.field_type());
            let _cursor_seq = t.field("cursor_seq", field_i64().build());
            t.primary_key("channel_pull_cursor_pk", &[&account_id, &identity, &channel, &source]);
        }
        {
            let t = v.table("session");
            let id = t.field("id", field_str().build());
            let _external_id = t.field("external_id", field_str().build());
            let _last_used = t.field("last_used", time_ms_t.field_type());
            t.primary_key("session_pk", &[&id]);
        }
        version_2 = v.build();
    }
    let version_3;
    {
        let v = Version::new();
        let account_id_t =
            v
                .custom_type("account_id_t")
                .rust_type("crate::interface::db::DbAccountId")
                .base_type(type_i64().build());
        let identity_id_t =
            v
                .custom_type("identity_id_t")
                .rust_type("crate::interface::db::DbIdentity")
                .base_type(type_str().build());
        let identity_secret_t =
            v
                .custom_type("identity_secret_t")
                .rust_type("crate::interface::db::DbIdentitySecret")
                .base_type(type_str().build());
        let refresh_tls_state_t =
            v
                .custom_type("refresh_tls_state_t")
                .rust_type("spaghettinuum_native::interface::stored::self_tls::RefreshTlsState")
                .base_type(type_str().opt().build());
        let channelgroup_id_t =
            v
                .custom_type("channelgroup_id_t")
                .rust_type("crate::interface::db::DbChannelGroupId")
                .base_type(type_i64().build());
        let channel_id_t =
            v
                .custom_type("channel_id_t")
                .rust_type("crate::interface::db::DbChannelId")
                .base_type(type_str().build());
        let time_ms_t =
            v.custom_type("time_ms_t").rust_type("crate::interface::db::DbTimeMs").base_type(type_i64().build());
        {
            let t = v.table("account");
            let _rowid = t.rowid_field(None);
            let id = t.field("external_id", field_str().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            t.unique_index("account_external_id", &[&id]);
        }
        {
            let t = v.table("identity");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            let _secret = t.field("secret", identity_secret_t.field_type());
            let _tls_state = t.field("tls_state", refresh_tls_state_t.field_type());
            t.primary_key("identity_pk", &[&account_id, &id]);
            t.unique_index("identity_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channelgroup");
            let account_id = t.field("account_id", account_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            t.unique_index("channelgroup_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channel");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let id = t.field("id", channel_id_t.field_type());
            let _local_identity = t.field("local_identity", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let mut channel_group_ft = channelgroup_id_t.field_type();
            channel_group_ft.type_.opt = true;
            let _channel_group = t.field("channel_group", channel_group_ft);
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _page_size = t.field("page_size", field_i64().build());
            let _authenticated = t.field("authenticated", field_bool().build());
            let _public_read = t.field("public_read", field_bool().build());
            let _public_post = t.field("public_post", field_bool().build());
            let _retention_secs = t.field("retention_secs", field_i64().opt().build());
            let _retention_mode =
                t.field(
                    "retention_mode",
                    field_str().migrate_fill(SerialExpr::LitString("all".to_string())).build(),
                );
            let _backfill_done = t.field("backfill_done", field_bool().build());
            let _latest_seq = t.field("latest_seq", field_i64().opt().build());
            let _latest_snap_offset = t.field("latest_snap_offset", field_i64().opt().build());
            let _floor_snap_offset = t.field("floor_snap_offset", field_i64().opt().build());
            let _next_sender_unique = t.field("next_sender_unique", field_i64().build());
            let _deleted = t.field("deleted", field_type_opt(time_ms_t.field_type()));
            t.primary_key("channel_pk", &[&account_id, &identity, &id]);
            t.unique_index("channel_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_member");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let member = t.field("member", identity_id_t.field_type());
            let _confirmed = t.field("confirmed", field_bool().build());
            let _can_post = t.field("can_post", field_bool().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            t.primary_key("channel_member_pk", &[&account_id, &identity, &channel, &member]);
        }
        {
            let t = v.table("identity_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            t.unique_index("identity_invite_token", &[&identity, &token]);
            t.unique_index("identity_invite_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            let _can_post = t.field("can_post", field_bool().build());
            t.unique_index("channel_invite_token", &[&identity, &channel, &token]);
            t.unique_index("channel_invite_channel_idem", &[&account_id, &identity, &channel, &idem]);
        }
        {
            let t = v.table("message");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let seq = t.field("seq", field_i64().build());
            let snap_offset = t.field("snap_offset", field_i64().opt().build());
            let _receive_time = t.field("receive_time", time_ms_t.field_type());
            let client_id = t.field("client_id", field_str().opt().build());
            let _raw = t.field("raw", field_str().build());
            let _body_text = t.field("body_text", field_str().build());
            let _rel_type = t.field("rel_type", field_str().opt().build());
            let mut rel_target_sender_ft = identity_id_t.field_type();
            rel_target_sender_ft.type_.opt = true;
            let _rel_target_sender = t.field("rel_target_sender", rel_target_sender_ft);
            let _rel_target_unique = t.field("rel_target_unique", field_i64().opt().build());
            t.primary_key("message_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("message_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
            t.unique_index("message_seq", &[&account_id, &identity, &channel, &seq]);
            t.unique_index("message_snap_offset", &[&account_id, &identity, &channel, &snap_offset]);
        }
        {
            let t = v.table("outbox");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let client_id = t.field("client_id", field_str().build());
            let _raw = t.field("raw", field_str().build());
            let _created = t.field("created", time_ms_t.field_type());
            t.primary_key("outbox_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("outbox_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
        }
        {
            let t = v.table("server_settings");
            let key = t.field("key", field_str().build());
            let _value = t.field("value", field_str().build());
            t.primary_key("server_settings_pk", &[&key]);
        }
        {
            let t = v.table("contact");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            t.primary_key("contact_pk", &[&account_id, &id]);
        }
        {
            let t = v.table("file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            let mut created_ft = time_ms_t.field_type();
            created_ft.migration_default = Some(SerialExpr::LitI64(0));
            let _created = t.field("created", created_ft);
            t.primary_key("file_pk", &[&account_id, &hash]);
        }
        {
            let t = v.table("push_subscription");
            let account_id = t.field("account_id", account_id_t.field_type());
            let endpoint = t.field("endpoint", field_str().build());
            let _data = t.field("data", field_str().build());
            t.primary_key("push_subscription_pk", &[&account_id, &endpoint]);
        }
        {
            let t = v.table("channel_pull_cursor");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", identity_id_t.field_type());
            let _cursor_seq = t.field("cursor_seq", field_i64().build());
            t.primary_key("channel_pull_cursor_pk", &[&account_id, &identity, &channel, &source]);
        }
        {
            let t = v.table("session");
            let id = t.field("id", field_str().build());
            let _external_id = t.field("external_id", field_str().build());
            let _last_used = t.field("last_used", time_ms_t.field_type());
            t.primary_key("session_pk", &[&id]);
        }
        version_3 = v.build();
    }
    let version_4;
    {
        let v = Version::new();
        let account_id_t =
            v
                .custom_type("account_id_t")
                .rust_type("crate::interface::db::DbAccountId")
                .base_type(type_i64().build());
        let identity_id_t =
            v
                .custom_type("identity_id_t")
                .rust_type("crate::interface::db::DbIdentity")
                .base_type(type_str().build());
        let identity_secret_t =
            v
                .custom_type("identity_secret_t")
                .rust_type("crate::interface::db::DbIdentitySecret")
                .base_type(type_str().build());
        let refresh_tls_state_t =
            v
                .custom_type("refresh_tls_state_t")
                .rust_type("spaghettinuum_native::interface::stored::self_tls::RefreshTlsState")
                .base_type(type_str().opt().build());
        let channelgroup_id_t =
            v
                .custom_type("channelgroup_id_t")
                .rust_type("crate::interface::db::DbChannelGroupId")
                .base_type(type_i64().build());
        let channel_id_t =
            v
                .custom_type("channel_id_t")
                .rust_type("crate::interface::db::DbChannelId")
                .base_type(type_str().build());
        let time_ms_t =
            v.custom_type("time_ms_t").rust_type("crate::interface::db::DbTimeMs").base_type(type_i64().build());
        {
            let t = v.table("account");
            let _rowid = t.rowid_field(None);
            let id = t.field("external_id", field_str().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            t.unique_index("account_external_id", &[&id]);
        }
        {
            let t = v.table("identity");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            let _secret = t.field("secret", identity_secret_t.field_type());
            let _tls_state = t.field("tls_state", refresh_tls_state_t.field_type());
            t.primary_key("identity_pk", &[&account_id, &id]);
            t.unique_index("identity_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channelgroup");
            let account_id = t.field("account_id", account_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            t.unique_index("channelgroup_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channel");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let id = t.field("id", channel_id_t.field_type());
            let _local_identity = t.field("local_identity", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let mut channel_group_ft = channelgroup_id_t.field_type();
            channel_group_ft.type_.opt = true;
            let _channel_group = t.field("channel_group", channel_group_ft);
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _page_size = t.field("page_size", field_i64().build());
            let _authenticated = t.field("authenticated", field_bool().build());
            let _public_read = t.field("public_read", field_bool().build());
            let _public_post = t.field("public_post", field_bool().build());
            let _retention_secs = t.field("retention_secs", field_i64().opt().build());
            let _retention_mode =
                t.field(
                    "retention_mode",
                    field_str().migrate_fill(SerialExpr::LitString("all".to_string())).build(),
                );
            let _backfill_done = t.field("backfill_done", field_bool().build());
            let _latest_seq = t.field("latest_seq", field_i64().opt().build());
            let _latest_snap_offset = t.field("latest_snap_offset", field_i64().opt().build());
            let _floor_snap_offset = t.field("floor_snap_offset", field_i64().opt().build());
            let _next_sender_unique = t.field("next_sender_unique", field_i64().build());
            let _deleted = t.field("deleted", field_type_opt(time_ms_t.field_type()));
            t.primary_key("channel_pk", &[&account_id, &identity, &id]);
            t.unique_index("channel_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_member");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let member = t.field("member", identity_id_t.field_type());
            let _confirmed = t.field("confirmed", field_bool().build());
            let _can_post = t.field("can_post", field_bool().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            t.primary_key("channel_member_pk", &[&account_id, &identity, &channel, &member]);
        }
        {
            let t = v.table("identity_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            t.unique_index("identity_invite_token", &[&identity, &token]);
            t.unique_index("identity_invite_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            let _can_post = t.field("can_post", field_bool().build());
            t.unique_index("channel_invite_token", &[&identity, &channel, &token]);
            t.unique_index("channel_invite_channel_idem", &[&account_id, &identity, &channel, &idem]);
        }
        {
            let t = v.table("message");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let seq = t.field("seq", field_i64().build());
            let snap_offset = t.field("snap_offset", field_i64().opt().build());
            let _receive_time = t.field("receive_time", time_ms_t.field_type());
            let client_id = t.field("client_id", field_str().opt().build());
            let _raw = t.field("raw", field_str().build());
            let _blocks =
                t.field("blocks", field_str().migrate_fill(SerialExpr::LitString("[]".to_string())).build());
            let _rel_type = t.field("rel_type", field_str().opt().build());
            let mut rel_target_sender_ft = identity_id_t.field_type();
            rel_target_sender_ft.type_.opt = true;
            let _rel_target_sender = t.field("rel_target_sender", rel_target_sender_ft);
            let _rel_target_unique = t.field("rel_target_unique", field_i64().opt().build());
            t.primary_key("message_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("message_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
            t.unique_index("message_seq", &[&account_id, &identity, &channel, &seq]);
            t.unique_index("message_snap_offset", &[&account_id, &identity, &channel, &snap_offset]);
        }
        {
            let t = v.table("outbox");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let client_id = t.field("client_id", field_str().build());
            let _raw = t.field("raw", field_str().build());
            let _created = t.field("created", time_ms_t.field_type());
            t.primary_key("outbox_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("outbox_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
        }
        {
            let t = v.table("server_settings");
            let key = t.field("key", field_str().build());
            let _value = t.field("value", field_str().build());
            t.primary_key("server_settings_pk", &[&key]);
        }
        {
            let t = v.table("contact");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            t.primary_key("contact_pk", &[&account_id, &id]);
        }
        {
            let t = v.table("file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            let mut created_ft = time_ms_t.field_type();
            created_ft.migration_default = Some(SerialExpr::LitI64(0));
            let _created = t.field("created", created_ft);
            t.primary_key("file_pk", &[&account_id, &hash]);
        }
        {
            let t = v.table("push_subscription");
            let account_id = t.field("account_id", account_id_t.field_type());
            let endpoint = t.field("endpoint", field_str().build());
            let _data = t.field("data", field_str().build());
            t.primary_key("push_subscription_pk", &[&account_id, &endpoint]);
        }
        {
            let t = v.table("channel_pull_cursor");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", identity_id_t.field_type());
            let _cursor_seq = t.field("cursor_seq", field_i64().build());
            t.primary_key("channel_pull_cursor_pk", &[&account_id, &identity, &channel, &source]);
        }
        {
            let t = v.table("session");
            let id = t.field("id", field_str().build());
            let _external_id = t.field("external_id", field_str().build());
            let _last_used = t.field("last_used", time_ms_t.field_type());
            t.primary_key("session_pk", &[&id]);
        }
        {
            let t = v.table("message_file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", field_str().build());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let hash = t.field("hash", field_str().build());
            t.primary_key(
                "message_file_pk",
                &[&account_id, &identity, &channel, &source, &sender, &sender_unique, &hash],
            );
            t.index("message_file_account_hash", &[&account_id, &hash]);
            t.index("message_file_channel_hash", &[&identity, &channel, &hash]);
        }
        {
            let t = v.table("file_cache");
            let owner = t.field("owner", identity_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let thumbnail = t.field("thumbnail", field_bool().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            let last_access = t.field("last_access", time_ms_t.field_type());
            t.primary_key("file_cache_pk", &[&owner, &hash, &thumbnail]);
            t.index("file_cache_last_access", &[&last_access]);
        }
        {
            let t = v.table("file_cache_access");
            let account_id = t.field("account_id", account_id_t.field_type());
            let owner = t.field("owner", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            t.primary_key("file_cache_access_pk", &[&account_id, &owner, &channel, &hash]);
            t.index("file_cache_access_owner_hash", &[&owner, &hash]);
        }
        version_4 = v.build();
    }
    let version_5;
    {
        let v = Version::new();
        let account_id_t =
            v
                .custom_type("account_id_t")
                .rust_type("crate::interface::db::DbAccountId")
                .base_type(type_i64().build());
        let identity_id_t =
            v
                .custom_type("identity_id_t")
                .rust_type("crate::interface::db::DbIdentity")
                .base_type(type_str().build());
        let identity_secret_t =
            v
                .custom_type("identity_secret_t")
                .rust_type("crate::interface::db::DbIdentitySecret")
                .base_type(type_str().build());
        let refresh_tls_state_t =
            v
                .custom_type("refresh_tls_state_t")
                .rust_type("spaghettinuum_native::interface::stored::self_tls::RefreshTlsState")
                .base_type(type_str().opt().build());
        let channelgroup_id_t =
            v
                .custom_type("channelgroup_id_t")
                .rust_type("crate::interface::db::DbChannelGroupId")
                .base_type(type_i64().build());
        let channel_id_t =
            v
                .custom_type("channel_id_t")
                .rust_type("crate::interface::db::DbChannelId")
                .base_type(type_str().build());
        let time_ms_t =
            v.custom_type("time_ms_t").rust_type("crate::interface::db::DbTimeMs").base_type(type_i64().build());
        {
            let t = v.table("account");
            let _rowid = t.rowid_field(None);
            let id = t.field("external_id", field_str().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            t.unique_index("account_external_id", &[&id]);
        }
        {
            let t = v.table("identity");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _soft_deleted_at = t.field("soft_deleted_at", field_type_opt(time_ms_t.field_type()));
            let _secret = t.field("secret", identity_secret_t.field_type());
            let _tls_state = t.field("tls_state", refresh_tls_state_t.field_type());
            t.primary_key("identity_pk", &[&account_id, &id]);
            t.unique_index("identity_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channelgroup");
            let account_id = t.field("account_id", account_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            t.unique_index("channelgroup_account_idem", &[&account_id, &idem]);
        }
        {
            let t = v.table("channel");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let id = t.field("id", channel_id_t.field_type());
            let _local_identity = t.field("local_identity", identity_id_t.field_type());
            let idem = t.field("idem", field_str().build());
            let mut channel_group_ft = channelgroup_id_t.field_type();
            channel_group_ft.type_.opt = true;
            let _channel_group = t.field("channel_group", channel_group_ft);
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            let _page_size = t.field("page_size", field_i64().build());
            let _authenticated = t.field("authenticated", field_bool().build());
            let _public_read = t.field("public_read", field_bool().build());
            let _public_post = t.field("public_post", field_bool().build());
            let _retention_secs = t.field("retention_secs", field_i64().opt().build());
            let _retention_mode =
                t.field(
                    "retention_mode",
                    field_str().migrate_fill(SerialExpr::LitString("all".to_string())).build(),
                );
            let _backfill_done = t.field("backfill_done", field_bool().build());
            let _latest_seq = t.field("latest_seq", field_i64().opt().build());
            let _latest_snap_offset = t.field("latest_snap_offset", field_i64().opt().build());
            let _floor_snap_offset = t.field("floor_snap_offset", field_i64().opt().build());
            let _next_sender_unique = t.field("next_sender_unique", field_i64().build());
            let _deleted = t.field("deleted", field_type_opt(time_ms_t.field_type()));
            let _reminders = t.field("reminders", field_bool().migrate_fill(SerialExpr::LitBool(false)).build());
            let _reminder_lead_minutes =
                t.field("reminder_lead_minutes", field_i64().migrate_fill(SerialExpr::LitI64(15)).build());
            t.primary_key("channel_pk", &[&account_id, &identity, &id]);
            t.unique_index("channel_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_member");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let member = t.field("member", identity_id_t.field_type());
            let _confirmed = t.field("confirmed", field_bool().build());
            let _can_post = t.field("can_post", field_bool().build());
            let _memo_short = t.field("memo_short", field_str().build());
            let _memo_long = t.field("memo_long", field_str().build());
            t.primary_key("channel_member_pk", &[&account_id, &identity, &channel, &member]);
        }
        {
            let t = v.table("identity_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            t.unique_index("identity_invite_token", &[&identity, &token]);
            t.unique_index("identity_invite_account_identity_idem", &[&account_id, &identity, &idem]);
        }
        {
            let t = v.table("channel_invite");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let _rowid = t.rowid_field(None);
            let idem = t.field("idem", field_str().build());
            let token = t.field("token", field_str().build());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _shown_name = t.field("shown_name", field_str().opt().build());
            let _shown_description = t.field("shown_description", field_str().opt().build());
            let _single_use = t.field("single_use", field_bool().build());
            let _expiry = t.field("expiry", field_type_opt(time_ms_t.field_type()));
            let _used_count = t.field("used_count", field_i64().build());
            let _can_post = t.field("can_post", field_bool().build());
            t.unique_index("channel_invite_token", &[&identity, &channel, &token]);
            t.unique_index("channel_invite_channel_idem", &[&account_id, &identity, &channel, &idem]);
        }
        {
            let t = v.table("message");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let seq = t.field("seq", field_i64().build());
            let snap_offset = t.field("snap_offset", field_i64().opt().build());
            let _receive_time = t.field("receive_time", time_ms_t.field_type());
            let client_id = t.field("client_id", field_str().opt().build());
            let _raw = t.field("raw", field_str().build());
            let _blocks =
                t.field("blocks", field_str().migrate_fill(SerialExpr::LitString("[]".to_string())).build());
            let _rel_type = t.field("rel_type", field_str().opt().build());
            let mut rel_target_sender_ft = identity_id_t.field_type();
            rel_target_sender_ft.type_.opt = true;
            let _rel_target_sender = t.field("rel_target_sender", rel_target_sender_ft);
            let _rel_target_unique = t.field("rel_target_unique", field_i64().opt().build());
            t.primary_key("message_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("message_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
            t.unique_index("message_seq", &[&account_id, &identity, &channel, &seq]);
            t.unique_index("message_snap_offset", &[&account_id, &identity, &channel, &snap_offset]);
        }
        {
            let t = v.table("outbox");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let client_id = t.field("client_id", field_str().build());
            let _raw = t.field("raw", field_str().build());
            let _created = t.field("created", time_ms_t.field_type());
            t.primary_key("outbox_pk", &[&account_id, &identity, &channel, &sender, &sender_unique]);
            t.unique_index("outbox_client_id", &[&account_id, &identity, &channel, &sender, &client_id]);
        }
        {
            let t = v.table("server_settings");
            let key = t.field("key", field_str().build());
            let _value = t.field("value", field_str().build());
            t.primary_key("server_settings_pk", &[&key]);
        }
        {
            let t = v.table("contact");
            let account_id = t.field("account_id", account_id_t.field_type());
            let id = t.field("id", identity_id_t.field_type());
            let _name = t.field("name", field_str().build());
            let _description = t.field("description", field_str().build());
            let _portrait = t.field("portrait", field_str().opt().build());
            t.primary_key("contact_pk", &[&account_id, &id]);
        }
        {
            let t = v.table("file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            let mut created_ft = time_ms_t.field_type();
            created_ft.migration_default = Some(SerialExpr::LitI64(0));
            let _created = t.field("created", created_ft);
            t.primary_key("file_pk", &[&account_id, &hash]);
        }
        {
            let t = v.table("push_subscription");
            let account_id = t.field("account_id", account_id_t.field_type());
            let endpoint = t.field("endpoint", field_str().build());
            let _data = t.field("data", field_str().build());
            t.primary_key("push_subscription_pk", &[&account_id, &endpoint]);
        }
        {
            let t = v.table("channel_pull_cursor");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", identity_id_t.field_type());
            let _cursor_seq = t.field("cursor_seq", field_i64().build());
            t.primary_key("channel_pull_cursor_pk", &[&account_id, &identity, &channel, &source]);
        }
        {
            let t = v.table("session");
            let id = t.field("id", field_str().build());
            let _external_id = t.field("external_id", field_str().build());
            let _last_used = t.field("last_used", time_ms_t.field_type());
            t.primary_key("session_pk", &[&id]);
        }
        {
            let t = v.table("message_file");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let source = t.field("source", field_str().build());
            let sender = t.field("sender", identity_id_t.field_type());
            let sender_unique = t.field("sender_unique", field_i64().build());
            let hash = t.field("hash", field_str().build());
            t.primary_key(
                "message_file_pk",
                &[&account_id, &identity, &channel, &source, &sender, &sender_unique, &hash],
            );
            t.index("message_file_account_hash", &[&account_id, &hash]);
            t.index("message_file_channel_hash", &[&identity, &channel, &hash]);
        }
        {
            let t = v.table("file_cache");
            let owner = t.field("owner", identity_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            let thumbnail = t.field("thumbnail", field_bool().build());
            let _mime_type = t.field("mime_type", field_str().build());
            let _size = t.field("size", field_i64().build());
            let last_access = t.field("last_access", time_ms_t.field_type());
            t.primary_key("file_cache_pk", &[&owner, &hash, &thumbnail]);
            t.index("file_cache_last_access", &[&last_access]);
        }
        {
            let t = v.table("file_cache_access");
            let account_id = t.field("account_id", account_id_t.field_type());
            let owner = t.field("owner", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let hash = t.field("hash", field_str().build());
            t.primary_key("file_cache_access_pk", &[&account_id, &owner, &channel, &hash]);
            t.index("file_cache_access_owner_hash", &[&owner, &hash]);
        }
        {
            let t = v.table("message_event");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let root_sender = t.field("root_sender", identity_id_t.field_type());
            let root_unique = t.field("root_unique", field_i64().build());
            let block_index = t.field("block_index", field_i64().build());
            let _version_seq = t.field("version_seq", field_i64().build());
            let envelope_start = t.field("envelope_start", time_ms_t.field_type());
            let _envelope_end = t.field("envelope_end", field_type_opt(time_ms_t.field_type()));
            let _event = t.field("event", field_str().build());
            t.primary_key(
                "message_event_pk",
                &[&account_id, &identity, &channel, &root_sender, &root_unique, &block_index],
            );
            t.index("message_event_envelope", &[&account_id, &identity, &channel, &envelope_start]);
        }
        {
            let t = v.table("reminder_sent");
            let account_id = t.field("account_id", account_id_t.field_type());
            let identity = t.field("identity", identity_id_t.field_type());
            let channel = t.field("channel", channel_id_t.field_type());
            let root_sender = t.field("root_sender", identity_id_t.field_type());
            let root_unique = t.field("root_unique", field_i64().build());
            let block_index = t.field("block_index", field_i64().build());
            let occurrence_start = t.field("occurrence_start", time_ms_t.field_type());
            t.primary_key(
                "reminder_sent_pk",
                &[&account_id, &identity, &channel, &root_sender, &root_unique, &block_index, &occurrence_start],
            );
            t.index("reminder_sent_occurrence_start", &[&occurrence_start]);
        }
        version_5 = v.build();
    }
    match good_ormning::sqlite::generate(GenerateArgs {
        versions: vec![
            (0usize, version_0),
            (1usize, version_1),
            (2usize, version_2),
            (3usize, version_3),
            (4usize, version_4),
            (5usize, version_5),
        ],
        ..Default::default()
    }) {
        Ok(_) => { },
        Err(e) => {
            for e in e {
                eprintln!(" - {}", e);
            }
            panic!("Generate failed.");
        },
    };
}
