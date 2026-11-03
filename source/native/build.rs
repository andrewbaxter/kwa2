use good_ormning::sqlite::{
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

fn version_0() -> Version {
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
        v
            .custom_type("time_ms_t")
            .rust_type("crate::interface::db::DbTimeMs")
            .base_type(type_i64().build());
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

    return v.build();
}

fn version_1() -> Version {
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
        v
            .custom_type("time_ms_t")
            .rust_type("crate::interface::db::DbTimeMs")
            .base_type(type_i64().build());
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

    return v.build();
}

fn version_2() -> Version {
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
        v
            .custom_type("time_ms_t")
            .rust_type("crate::interface::db::DbTimeMs")
            .base_type(type_i64().build());
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

    return v.build();
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    match good_ormning::sqlite::generate(GenerateArgs {
        versions: vec![(0usize, version_0()), (1usize, version_1()), (2usize, version_2())],
        ..Default::default()
    }) {
        Ok(_) => {},
        Err(e) => {
            for e in e {
                eprintln!(" - {}", e);
            }
            panic!("Generate failed.");
        },
    };
}
