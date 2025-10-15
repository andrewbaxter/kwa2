use {
    good_ormning::sqlite::{
        new_delete,
        new_insert,
        new_select,
        new_select_body,
        new_update,
        query::{
            expr::{
                BinOp,
                Binding,
                Expr,
            },
            helpers::{
                expr_and,
                expr_field_eq,
                expr_field_lt,
                expr_or,
                fn_max,
                set_field,
            },
            insert::InsertConflict,
            select_body::Order,
            utils::{
                CteBuilder,
                With,
            },
        },
        schema::{
            constraint::{
                ConstraintType::{
                    self,
                    PrimaryKey,
                },
                PrimaryKeyDef,
            },
            field::{
                field_bool,
                field_i64,
                field_str,
                field_utctime_ms,
                field_utctime_s,
                FieldType,
            },
        },
        types::{
            type_i64,
            type_str,
            Type,
            TypeBuilder,
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
    let account_internal_id0 = "crate::interface::shared::AccountId";
    let account_internal_id_t = type_i64().custom(account_internal_id0).build();
    let identity_internal_id0 = "crate::interface::shared::IdentityInternalId";
    let identity_internal_id_t = type_i64().custom(identity_internal_id0).build();
    let channel_internal_id0 = "crate::interface::shared::ChannelInternalId";
    let channel_internal_id_t = type_i64().custom(channel_internal_id0).build();
    let channel_id_t = type_str().custom("crate::interface::shared::ChannelId").build();
    let new_message_id = |opt| {
        let mut out = type_str().custom("crate::interface::shared::MessageId");
        if opt {
            out = out.opt();
        }
        return out.build();
    };
    let message_id = new_message_id(false);
    let identity_id_t0 = "spaghettinuum::interface::stored::identity::Identity";
    let identity_id_t = field_i64().custom(identity_id_t0).build();
    let identity_id_opt_t = field_i64().custom(identity_id_t0).opt().build();

    // Accounts
    {
        let t = latest_version.table("zQLEK3CT0", "account");
        let internal_id = t.rowid_field(&mut latest_version, Some(account_internal_id0.to_string()));
        let id = t.field(&mut latest_version, "zSZVNBP0E", "external_id", field_str().build());
        let soft_deleted_at =
            t.field(&mut latest_version, "zAFX0RY7Y", "soft_deleted_at", field_utctime_s().opt().build());
        t.index("zF2FTSZMJ", "account_external_id", &[&id]).unique().build(&mut latest_version);
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
        let internal_id = t.rowid_field(&mut latest_version, Some(identity_internal_id0.to_string()));
        let account_internal_id =
            t.field(&mut latest_version, "zLQI9HQUQ", "account", FieldType::with(&account_internal_id_t));
        let identity = t.field(&mut latest_version, "zQXLIG7CM", "identity", identity_id_t.clone());
        let description = t.field(&mut latest_version, "zSZVNBP0E", "description", field_str().build());
        let soft_deleted_at =
            t.field(&mut latest_version, "zAFX0RY7Y", "soft_deleted_at", field_utctime_s().opt().build());
        let secret =
            t.field(
                &mut latest_version,
                "zII52SWQB",
                "secret",
                field_str().custom("crate::interface::db::DbIdentitySecret").build(),
            );
        t
            .index("zEC7EPI2R", "identity_account_identity", &[&account_internal_id, &identity])
            .build(&mut latest_version);
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("account", &account_internal_id),
                    set_field("identity", &identity),
                    set_field("description", &description),
                    set_field("secret", &secret),
                    (soft_deleted_at.clone(), Expr::LitNull(soft_deleted_at.type_.type_.type_.clone()))
                ],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("identity_ensure", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("account", &account_internal_id))
                .return_fields(&[&internal_id, &identity, &description])
                .build_query("identity_list", QueryResCount::Many),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("identity", &internal_id))
                .return_fields(&[&secret])
                .build_query("identity_get_secret", QueryResCount::One),
        );
        queries.push(
            new_update(&t, vec![set_field("soft_deleted_at", &soft_deleted_at)])
                .where_(expr_field_eq("internal_id", &internal_id))
                .build_query("identity_soft_delete", QueryResCount::None),
        );
        let where_soft_deleted_before = expr_and(vec![
            //. .
            Expr::BinOp {
                left: Box::new(Expr::field(&soft_deleted_at)),
                op: BinOp::IsNot,
                right: Box::new(Expr::LitNull(soft_deleted_at.type_.type_.type_.clone())),
            },
            expr_field_lt("soft_deleted_at", &soft_deleted_at)
        ]);
        queries.push(
            new_select(&t)
                .where_(where_soft_deleted_before.clone())
                .return_field(&internal_id)
                .build_query("identity_list_soft_deleted_before", QueryResCount::Many),
        );
        queries.push(
            new_delete(&t)
                .where_(where_soft_deleted_before)
                .build_query("identity_hard_delete_before", QueryResCount::None),
        );
    }

    // Channels
    {
        let t = latest_version.table("z7B1CHM4F", "channel");
        let identity_internal_id =
            t.field(
                &mut latest_version,
                "zBA4NRQ76",
                "identity_internal",
                FieldType::with(&identity_internal_id_t),
            );
        let internal_id = t.rowid_field(&mut latest_version, Some(channel_internal_id0.to_string()));
        let id = t.field(&mut latest_version, "zII52SWQB", "id", FieldType::with(&channel_id_t));
        let description = t.field(&mut latest_version, "zSZVNBP0E", "description", field_str().build());
        let idem = t.field(&mut latest_version, "zZ6KUV5P0", "idem", field_str().build());
        let soft_deleted_at = t.field(&mut latest_version, "zAFX0RY7Y", "deleted", field_utctime_s().opt().build());
        t
            .index("zLV14KK8R", "channel_identity_idem", &[&identity_internal_id, &idem])
            .unique()
            .build(&mut latest_version);
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("identity", &identity_internal_id),
                    set_field("idem", &idem),
                    set_field("id", &id),
                    set_field("description", &description),
                    (soft_deleted_at.clone(), Expr::LitNull(soft_deleted_at.type_.type_.type_.clone()))
                ],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("channel_ensure", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("identity", &identity_internal_id))
                .return_fields(&[&internal_id, &identity_internal_id, &id, &description])
                .build_query("identity_list", QueryResCount::Many),
        );
        queries.push(
            new_update(&t, vec![set_field("deleted", &soft_deleted_at)])
                .where_(expr_field_eq("internal_id", &internal_id))
                .build_query("identity_soft_delete", QueryResCount::None),
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
                .build_query("identity_list_soft_deleted_before", QueryResCount::Many),
        );
        queries.push(
            new_delete(&t)
                .where_(where_soft_deleted_before)
                .build_query("identity_hard_delete_before", QueryResCount::None),
        );
    }

    // Members
    {
        let t = latest_version.table("ywyc97a308uwk6", "member");
        let channel_internal_id =
            t.field(&mut latest_version, "zK21ECBE5", "channel", FieldType::with(&channel_internal_id_t));
        let identity_id = t.field(&mut latest_version, "zLQI9HQUQ", "identity", identity_id_t.clone());
    }

    // Identity invitations
    {
        let t = latest_version.table("zFFF18JKY", "identity_invitation");
        let identity_internal_id =
            t.field(&mut latest_version, "zK21ECBE5", "identity", FieldType::with(&identity_internal_id_t));
        let token = t.field(&mut latest_version, "zSZVNBP0E", "token", field_str().build());
        let description = t.field(&mut latest_version, "z0ZOJM2UT", "description", field_str().build());
        let allow_identity = t.field(&mut latest_version, "zII52SWQB", "allow_identity", identity_id_opt_t.clone());
        let single_use = t.field(&mut latest_version, "zGBXXPVPA", "single_use", field_bool().build());
        let expires = t.field(&mut latest_version, "zZ7H60J92", "expires", field_utctime_ms().opt().build());
    }

    // Channel invitations
    {
        let t = latest_version.table("zZIQTJ4XY", "channel_invitation");
        let channel_internal_id =
            t.field(&mut latest_version, "zK21ECBE5", "channel", FieldType::with(&channel_internal_id_t));
        let token = t.field(&mut latest_version, "zSZVNBP0E", "token", field_str().build());
        let description = t.field(&mut latest_version, "z0ZOJM2UT", "description", field_str().build());
        let allow_identity = t.field(&mut latest_version, "zII52SWQB", "allow_identity", identity_id_opt_t.clone());
        let single_use = t.field(&mut latest_version, "zHC8M7R34", "single_use", field_bool().build());
        let expires = t.field(&mut latest_version, "zZ7H60J92", "expires", field_utctime_ms().opt().build());
    }

    // Messages
    {
        let t = latest_version.table("zNKHCTSZK", "message");
        let account_id =
            t.field(&mut latest_version, "zLQI9HQUQ", "account_id", FieldType::with(&account_internal_id_t));
        let channel_id =
            t.field(&mut latest_version, "zK21ECBE5", "channel_id", FieldType::with(&channel_internal_id_t));
        let message_id = t.field(&mut latest_version, "zQCUZVVKA", "message_id", FieldType::with(&message_id));
        let reply_id =
            t.field(&mut latest_version, "z56XIFHFF", "reply_id", FieldType::with(&new_message_id(true)));
        let message =
            t.field(
                &mut latest_version,
                "zII52SWQB",
                "message",
                field_str().custom("crate::interface::shared::Message").build(),
            );
        let deleted = t.field(&mut latest_version, "zAFX0RY7Y", "deleted", field_utctime_s().opt().build());
    }

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
