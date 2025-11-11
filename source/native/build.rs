use {
    good_ormning::sqlite::{
        new_delete,
        new_insert,
        new_select,
        new_update,
        query::{
            expr::{
                BinOp,
                Expr,
            },
            helpers::{
                expr_and,
                expr_field_eq,
                expr_field_lt,
                set_field,
            },
            insert::InsertConflict,
        },
        schema::{
            constraint::{
                ConstraintType::PrimaryKey,
                PrimaryKeyDef,
            },
            field::{
                field_i64,
                field_str,
                field_utctime_s_jiff,
                FieldType,
            },
        },
        types::{
            type_str,
        },
        QueryResCount,
        Version,
    },
    std::{
        env,
        path::PathBuf,
    },
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(&env::var("OUT_DIR").unwrap());
    let mut latest_version = Version::default();
    let mut queries = vec![];
    let account_id0 = "crate::interface::db::DbAccountId";
    let account_id_t = field_i64().custom(account_id0).build();
    let identity_secret_t0 = "crate::interface::db::DbIdentitySecret";
    let identity_id_t0 = "crate::interface::db::DbIdentity";
    let identity_id_t = field_str().custom(identity_id_t0).build();
    let channelgroup_id0 = "crate::interface::db:DbChannelGroupId";
    let channel_id_t = type_str().custom("shared::interface::shared::ChannelId").build();

    // Accounts
    {
        let t = latest_version.table("zQLEK3CT0", "account");

        // Id
        let internal_id = t.rowid_field(&mut latest_version, Some(account_id0.to_string()));
        let id = t.field(&mut latest_version, "zSZVNBP0E", "external_id", field_str().build());

        // Internal
        let soft_deleted_at =
            t.field(&mut latest_version, "zAFX0RY7Y", "soft_deleted_at", field_utctime_s_jiff().opt().build());
        t.index("zF2FTSZMJ", "account_external_id", &[&id]).unique().build(&mut latest_version);

        // Queries
        queries.push(
            new_insert(&t, vec![set_field("external_id", &id)])
                .on_conflict(InsertConflict::DoNothing)
                .build_query("account_ensure", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("external_id", &id))
                .return_field(&internal_id)
                .build_query("account_get_by_external_id", QueryResCount::MaybeOne),
        );
        queries.push(
            new_update(&t, vec![set_field("deleted", &soft_deleted_at)])
                .where_(expr_field_eq("id", &internal_id))
                .build_query("account_soft_delete", QueryResCount::None),
        );
        let where_soft_deleted_before = expr_and(vec![
            //. .
            Expr::BinOp {
                left: Box::new(Expr::field(&soft_deleted_at)),
                op: BinOp::IsNot,
                right: Box::new(Expr::LitNull(soft_deleted_at.type_.type_.type_.clone())),
            },
            expr_field_lt("deleted", &soft_deleted_at)
        ]);
        queries.push(
            new_select(&t)
                .where_(where_soft_deleted_before.clone())
                .return_field(&internal_id)
                .return_field(&id)
                .build_query("account_list_soft_deleted_before", QueryResCount::Many),
        );
        queries.push(
            new_delete(&t)
                .where_(where_soft_deleted_before)
                .build_query("account_hard_delete_before", QueryResCount::None),
        );
    }

    // Identities
    {
        let t = latest_version.table("z1YCS4PD2", "identity");

        // Qualification
        let account_id = t.field(&mut latest_version, "zLQI9HQUQ", "account_id", account_id_t.clone());

        // Id
        let id = t.field(&mut latest_version, "zQXLIG7CM", "id", identity_id_t.clone());
        let idem = t.field(&mut latest_version, "zZ6KUV5P0", "idem", field_str().build());

        // Internal
        let memo_short = t.field(&mut latest_version, "zSZVNBP0E", "memo_short", field_str().build());
        let memo_long = t.field(&mut latest_version, "z2N4TX4BZ", "memo_long", field_str().build());
        let soft_deleted_at =
            t.field(&mut latest_version, "zAFX0RY7Y", "soft_deleted_at", field_utctime_s_jiff().opt().build());
        let secret =
            t.field(
                &mut latest_version,
                "zII52SWQB",
                "secret",
                field_str().custom(identity_secret_t0).build(),
            );

        // Indexes
        t.constraint(
            &mut latest_version,
            "zK5LPDHOO",
            "identity_pk",
            PrimaryKey(PrimaryKeyDef { fields: vec![account_id.clone(), id.clone()] }),
        );
        t.index("z4380ADDA", "identity_account_idem", &[&account_id, &idem]).build(&mut latest_version);

        // Queries
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("account", &account_id),
                    set_field("id", &id),
                    set_field("idem", &idem),
                    set_field("memo_short", &memo_short),
                    set_field("memo_long", &memo_long),
                    set_field("secret", &secret),
                    (soft_deleted_at.clone(), Expr::LitNull(soft_deleted_at.type_.type_.type_.clone()))
                ],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("identity_ensure", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("account", &account_id))
                .return_fields(&[&id, &idem, &memo_short, &memo_long])
                .build_query("identity_list", QueryResCount::Many),
        );
        //.        let where_id = expr_and(vec![expr_field_eq("account_id", &account_id), expr_field_eq("identity", &id)]);
        //.        queries.push(
        //.            new_select(&t)
        //.                .where_(where_id.clone())
        //.                .return_fields(&[&secret])
        //.                .build_query("identity_get_secret", QueryResCount::One),
        //.        );
        //.        queries.push(
        //.            new_update(&t, vec![set_field("soft_deleted_at", &soft_deleted_at)])
        //.                .where_(where_id.clone())
        //.                .build_query("identity_soft_delete", QueryResCount::None),
        //.        );
        //.        let where_soft_deleted_before = expr_and(vec![
        //.            //. .
        //.            Expr::BinOp {
        //.                left: Box::new(Expr::field(&soft_deleted_at)),
        //.                op: BinOp::IsNot,
        //.                right: Box::new(Expr::LitNull(soft_deleted_at.type_.type_.type_.clone())),
        //.            },
        //.            expr_field_lt("soft_deleted_at", &soft_deleted_at)
        //.        ]);
        //.        queries.push(
        //.            new_select(&t)
        //.                .where_(where_soft_deleted_before.clone())
        //.                .return_field(&account_id)
        //.                .return_field(&id)
        //.                .build_query("identity_list_soft_deleted_before", QueryResCount::Many),
        //.        );
        //.        queries.push(
        //.            new_delete(&t)
        //.                .where_(where_soft_deleted_before)
        //.                .build_query("identity_hard_delete_before", QueryResCount::None),
        //.        );
    }

    // Channel groups
    {
        let t = latest_version.table("z25K76CY2", "channelgroup");

        // Qualification
        let account_id = t.field(&mut latest_version, "zLQI9HQUQ", "account_id", account_id_t.clone());

        // Ids
        let id = t.rowid_field(&mut latest_version, Some(channelgroup_id0.to_string()));
        let idem = t.field(&mut latest_version, "zZ6KUV5P0", "idem", field_str().build());

        // Body
        let memo_short = t.field(&mut latest_version, "zSZVNBP0E", "memo_short", field_str().build());
        let memo_long = t.field(&mut latest_version, "z2N4TX4BZ", "memo_long", field_str().build());

        // Indexes
        t.index("zLV14KK8R", "channelgroup_account_idem", &[&account_id, &idem]).unique().build(&mut latest_version);

        // Queries
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("account", &account_id),
                    set_field("idem", &idem),
                    set_field("memo_short", &memo_short),
                    set_field("memo_long", &memo_long),
                ],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("channel_ensure", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("account", &account_id))
                .return_fields(&[&id, &idem, &memo_short, &memo_long])
                .build_query("channelgroup_list", QueryResCount::Many),
        );
        let where_id = expr_and(vec![expr_field_eq("account_id", &account_id), expr_field_eq("identity", &id)]);
        queries.push(new_delete(&t).where_(where_id).build_query("channelgroup_soft_delete", QueryResCount::None));
    }

    // Channels
    {
        let t = latest_version.table("z7B1CHM4F", "channel");

        // Qualification
        let account_id = t.field(&mut latest_version, "zLQI9HQUQ", "account_id", account_id_t.clone());
        let identity = t.field(&mut latest_version, "zPIABDQFI", "identity", identity_id_t.clone());

        // Ids
        let id = t.field(&mut latest_version, "zII52SWQB", "id", FieldType::with(&channel_id_t));
        let idem = t.field(&mut latest_version, "zZ6KUV5P0", "idem", field_str().build());

        // Body
        let channel_group =
            t.field(
                &mut latest_version,
                "zQRJN5PDV",
                "channel_group",
                field_i64().custom(&channelgroup_id0).opt().build(),
            );
        let memo_short = t.field(&mut latest_version, "zSZVNBP0E", "memo_short", field_str().build());
        let memo_long = t.field(&mut latest_version, "z2N4TX4BZ", "memo_long", field_str().build());
        let soft_deleted_at =
            t.field(&mut latest_version, "zAFX0RY7Y", "deleted", field_utctime_s_jiff().opt().build());

        // Indexes
        t.constraint(
            &mut latest_version,
            "zIYM9D9C5",
            "channel_pk",
            PrimaryKey(PrimaryKeyDef { fields: vec![account_id.clone(), identity.clone(), id.clone()] }),
        );
        t
            .index("zLV14KK8R", "channel_account_identity_idem", &[&account_id, &identity, &idem])
            .unique()
            .build(&mut latest_version);

        // Queries
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("account", &account_id),
                    set_field("identity", &identity),
                    set_field("idem", &idem),
                    set_field("id", &id),
                    set_field("channel_group", &channel_group),
                    set_field("memo_short", &memo_short),
                    set_field("memo_long", &memo_long),
                    (soft_deleted_at.clone(), Expr::LitNull(soft_deleted_at.type_.type_.type_.clone()))
                ],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("channel_ensure", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("account", &account_id))
                .return_fields(&[&identity, &id, &idem, &channel_group, &memo_short, &memo_long])
                .build_query("channel_list", QueryResCount::Many),
        );
        //.        let where_id = expr_and(vec![expr_field_eq("account_id", &account_id), expr_field_eq("id", &id)]);
        //.        queries.push(
        //.            new_update(&t, vec![set_field("deleted", &soft_deleted_at)])
        //.                .where_(where_id.clone())
        //.                .build_query("channel_soft_delete", QueryResCount::None),
        //.        );
        //.        let where_soft_deleted_before = expr_and(vec![
        //.            //. .
        //.            Expr::BinOp {
        //.                left: Box::new(Expr::field(&soft_deleted_at)),
        //.                op: BinOp::IsNot,
        //.                right: Box::new(Expr::LitNull(soft_deleted_at.type_.type_.type_.clone())),
        //.            },
        //.            expr_field_lt("deleted", &soft_deleted_at)
        //.        ]);
        //.        queries.push(
        //.            new_select(&t)
        //.                .where_(where_soft_deleted_before.clone())
        //.                .return_field(&internal_id)
        //.                .build_query("channel_list_soft_deleted_before", QueryResCount::Many),
        //.        );
        //.        queries.push(
        //.            new_delete(&t)
        //.                .where_(where_soft_deleted_before)
        //.                .build_query("channel_hard_delete_before", QueryResCount::None),
        //.        );
    }

    //. // Members
    //.    {
    //.        let t = latest_version.table("ywyc97a308uwk6", "member");
    //.        let channel_internal_id =
    //.            t.field(&mut latest_version, "zK21ECBE5", "channel", FieldType::with(&channel_internal_id_t));
    //.        let identity_id = t.field(&mut latest_version, "zLQI9HQUQ", "identity", identity_id_t.clone());
    //.    }
    //. 
    //.    // Identity invitations
    //.    {
    //.        let t = latest_version.table("zFFF18JKY", "identity_invitation");
    //.        let identity_internal_id =
    //.            t.field(&mut latest_version, "zK21ECBE5", "identity", FieldType::with(&identity_internal_id_t));
    //.        let token = t.field(&mut latest_version, "zSZVNBP0E", "token", field_str().build());
    //.        let memo_short = t.field(&mut latest_version, "z0ZOJM2UT", "memo_short", field_str().build());
    //.        let memo_long = t.field(&mut latest_version, "z2N4TX4BZ", "memo_long", field_str().build());
    //.        let allow_identity = t.field(&mut latest_version, "zII52SWQB", "allow_identity", identity_id_opt_t.clone());
    //.        let single_use = t.field(&mut latest_version, "zGBXXPVPA", "single_use", field_bool().build());
    //.        let expires = t.field(&mut latest_version, "zZ7H60J92", "expires", field_utctime_ms_jiff().opt().build());
    //.    }
    //. 
    //.    // Channel invitations
    //.    {
    //.        let t = latest_version.table("zZIQTJ4XY", "channel_invitation");
    //.        let channel_internal_id =
    //.            t.field(&mut latest_version, "zK21ECBE5", "channel", FieldType::with(&channel_internal_id_t));
    //.        let token = t.field(&mut latest_version, "zSZVNBP0E", "token", field_str().build());
    //.        let memo_short = t.field(&mut latest_version, "z0ZOJM2UT", "memo_short", field_str().build());
    //.        let memo_long = t.field(&mut latest_version, "z2N4TX4BZ", "memo_long", field_str().build());
    //.        let allow_identity = t.field(&mut latest_version, "zII52SWQB", "allow_identity", identity_id_opt_t.clone());
    //.        let single_use = t.field(&mut latest_version, "zHC8M7R34", "single_use", field_bool().build());
    //.        let expires = t.field(&mut latest_version, "zZ7H60J92", "expires", field_utctime_ms_jiff().opt().build());
    //.    }
    //. 
    //.    // Messages
    //.    {
    //.        let t = latest_version.table("zNKHCTSZK", "message");
    //.        let account_id = t.field(&mut latest_version, "zLQI9HQUQ", "account_id", FieldType::with(&account_id_t));
    //.        let channel_id =
    //.            t.field(&mut latest_version, "zK21ECBE5", "channel_id", FieldType::with(&channel_internal_id_t));
    //.        let message_id = t.field(&mut latest_version, "zQCUZVVKA", "message_id", FieldType::with(&message_id));
    //.        let reply_id =
    //.            t.field(&mut latest_version, "z56XIFHFF", "reply_id", FieldType::with(&new_message_id(true)));
    //.        let message =
    //.            t.field(
    //.                &mut latest_version,
    //.                "zII52SWQB",
    //.                "message",
    //.                field_str().custom("crate::interface::shared::Message").build(),
    //.            );
    //.        let deleted = t.field(&mut latest_version, "zAFX0RY7Y", "deleted", field_utctime_s_jiff().opt().build());
    //.    }
    // Generate
    match good_ormning::sqlite::generate(&root.join("src/db.rs"), vec![
        // Versions
        (0usize, latest_version)
    ], queries) {
        Ok(_) => { },
        Err(e) => {
            for e in e {
                eprintln!(" - {}", e);
            }
            panic!("Generate failed.");
        },
    };
}
