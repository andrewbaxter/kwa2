use good_ormning::sqlite::{
    schema::field::{
        field_i64,
        field_str,
        field_utctime_s_jiff,
    },
    types::{
        type_i64,
        type_str,
    },
    GenerateArgs,
    Version,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let latest_version = Version::new();

    // Custom types
    let account_id_t =
        latest_version
            .custom_type("account_id_t")
            .rust_type("crate::interface::db::DbAccountId")
            .base_type(type_i64().build());
    let identity_id_t =
        latest_version
            .custom_type("identity_id_t")
            .rust_type("crate::interface::db::DbIdentity")
            .base_type(type_str().build());
    let identity_secret_t =
        latest_version
            .custom_type("identity_secret_t")
            .rust_type("crate::interface::db::DbIdentitySecret")
            .base_type(type_str().build());
    let channelgroup_id_t =
        latest_version
            .custom_type("channelgroup_id_t")
            .rust_type("crate::interface::db::DbChannelGroupId")
            .base_type(type_i64().build());
    let channel_id_t =
        latest_version
            .custom_type("channel_id_t")
            .rust_type("crate::interface::db::DbChannelId")
            .base_type(type_str().build());

    // Accounts
    {
        let t = latest_version.table("account");
        let _rowid = t.rowid_field(None);
        let id = t.field("external_id", field_str().build());
        let _soft_deleted_at = t.field("soft_deleted_at", field_utctime_s_jiff().opt().build());
        t.unique_index("account_external_id", &[&id]);
    }

    // Identities
    {
        let t = latest_version.table("identity");
        let account_id = t.field("account_id", account_id_t.field_type());
        let id = t.field("id", identity_id_t.field_type());
        let idem = t.field("idem", field_str().build());
        let _memo_short = t.field("memo_short", field_str().build());
        let _memo_long = t.field("memo_long", field_str().build());
        let _soft_deleted_at = t.field("soft_deleted_at", field_utctime_s_jiff().opt().build());
        let _secret = t.field("secret", identity_secret_t.field_type());
        t.primary_key("identity_pk", &[&account_id, &id]);
        t.index("identity_account_idem", &[&account_id, &idem]);
    }

    // Channel groups
    {
        let t = latest_version.table("channelgroup");
        let account_id = t.field("account_id", account_id_t.field_type());
        let _rowid = t.rowid_field(None);
        let idem = t.field("idem", field_str().build());
        let _memo_short = t.field("memo_short", field_str().build());
        let _memo_long = t.field("memo_long", field_str().build());
        t.unique_index("channelgroup_account_idem", &[&account_id, &idem]);
    }

    // Channels
    {
        let t = latest_version.table("channel");
        let account_id = t.field("account_id", account_id_t.field_type());
        let identity = t.field("identity", identity_id_t.field_type());
        let id = t.field("id", channel_id_t.field_type());
        let idem = t.field("idem", field_str().build());
        let mut channel_group_ft = channelgroup_id_t.field_type();
        channel_group_ft.type_.opt = true;
        let _channel_group = t.field("channel_group", channel_group_ft);
        let _memo_short = t.field("memo_short", field_str().build());
        let _memo_long = t.field("memo_long", field_str().build());
        let _deleted = t.field("deleted", field_utctime_s_jiff().opt().build());
        t.primary_key("channel_pk", &[&account_id, &identity, &id]);
        t.unique_index("channel_account_identity_idem", &[&account_id, &identity, &idem]);
    }

    match good_ormning::sqlite::generate(GenerateArgs {
        versions: vec![(0usize, latest_version.build())],
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
