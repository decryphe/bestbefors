use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "translations",
            &[
                ("id", ColType::PkAuto),
                ("code", ColType::String),
                ("lang", ColType::String),
                ("text", ColType::String),
            ],
            &[],
        )
        .await?;

        create_table(
            m,
            "expiries",
            &[
                ("id", ColType::PkAuto),
                ("code", ColType::String),
                ("sqlite_modifier", ColType::String),
                ("sqlite_num_of_modifier", ColType::Integer),
            ],
            &[],
        )
        .await?;

        create_table(
            m,
            "intervals",
            &[
                ("id", ColType::PkAuto),
                ("code", ColType::String),
                ("sqlite_modifier", ColType::String),
                ("sqlite_num_of_modifier", ColType::Integer),
            ],
            &[],
        )
        .await?;

        create_table(
            m,
            "results",
            &[("id", ColType::PkAuto), ("code", ColType::String)],
            &[],
        )
        .await?;

        create_table(
            m,
            "checklists",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("description", ColType::TextNull),
            ],
            &[],
        )
        .await?;

        create_table(
            m,
            "checklist_steps",
            &[
                ("id", ColType::PkAuto),
                ("position", ColType::Integer),
                ("name", ColType::String),
                ("description", ColType::TextNull),
            ],
            &[("checklist", "")],
        )
        .await?;

        create_table(
            m,
            "executed_checklists",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("description", ColType::TextNull),
            ],
            &[],
        )
        .await?;

        create_table(
            m,
            "executed_checklist_steps",
            &[
                ("id", ColType::PkAuto),
                ("position", ColType::Integer),
                ("name", ColType::String),
                ("description", ColType::TextNull),
            ],
            &[("executed_checklist", "")],
        )
        .await?;

        create_table(
            m,
            "inventory_item_kinds",
            &[("id", ColType::PkAuto), ("name", ColType::StringUniq)],
            &[
                ("checklist", "default_checklist_id"),
                ("interval", "default_interval_id"),
                ("expiry", "default_expiry_id"),
            ],
        )
        .await?;

        create_table(
            m,
            "inventory_items",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::String),
                ("serial_number", ColType::StringNull),
                ("last_checked_at", ColType::TimestampWithTimeZoneNull),
                ("expiry", ColType::TimestampWithTimeZoneNull),
            ],
            &[
                ("inventory_item_kind", ""),
                ("checklist", ""),
                ("interval", ""),
            ],
        )
        .await?;

        create_table(
            m,
            "inventory_item_checks",
            &[
                ("id", ColType::PkAuto),
                ("finished", ColType::Boolean),
                ("checked_at", ColType::TimestampWithTimeZone),
                ("notes", ColType::TextNull),
            ],
            &[
                ("inventory_item", ""),
                ("executed_checklist", ""),
                ("user", "checked_by"),
                ("result", ""),
            ],
        )
        .await?;

        create_table(
            m,
            "inventory_item_check_steps",
            &[("id", ColType::PkAuto), ("notes", ColType::TextNull)],
            &[
                ("inventory_item_check", ""),
                ("executed_checklist_step", ""),
                ("result", ""),
            ],
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "inventory_item_check_steps").await?;
        drop_table(m, "inventory_item_checks").await?;
        drop_table(m, "inventory_items").await?;
        drop_table(m, "inventory_item_kinds").await?;
        drop_table(m, "executed_checklist_steps").await?;
        drop_table(m, "executed_checklists").await?;
        drop_table(m, "checklist_steps").await?;
        drop_table(m, "checklists").await?;
        drop_table(m, "results").await?;
        drop_table(m, "intervals").await?;
        drop_table(m, "expiries").await?;
        drop_table(m, "translations").await?;

        Ok(())
    }
}
