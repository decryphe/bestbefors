use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Alias::new("inventory_item_kind_metadata_fields"))
                .if_not_exists()
                .col(pk_auto(Alias::new("id")))
                .col(string(Alias::new("name")))
                .col(integer(Alias::new("position")))
                .col(
                    timestamp_with_time_zone(Alias::new("created_at"))
                        .default(Expr::current_timestamp()),
                )
                .col(
                    timestamp_with_time_zone(Alias::new("updated_at"))
                        .default(Expr::current_timestamp()),
                )
                .col(integer(Alias::new("inventory_item_kind_id")))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-kind-metadata-fields-item-kind")
                        .from(
                            Alias::new("inventory_item_kind_metadata_fields"),
                            Alias::new("inventory_item_kind_id"),
                        )
                        .to(Alias::new("inventory_item_kinds"), Alias::new("id"))
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        m.create_table(
            Table::create()
                .table(Alias::new("inventory_item_metadata_values"))
                .if_not_exists()
                .col(pk_auto(Alias::new("id")))
                .col(text(Alias::new("value")).default(""))
                .col(
                    timestamp_with_time_zone(Alias::new("created_at"))
                        .default(Expr::current_timestamp()),
                )
                .col(
                    timestamp_with_time_zone(Alias::new("updated_at"))
                        .default(Expr::current_timestamp()),
                )
                .col(integer(Alias::new("inventory_item_id")))
                .col(integer(Alias::new("inventory_item_kind_metadata_field_id")))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-item-metadata-values-item")
                        .from(
                            Alias::new("inventory_item_metadata_values"),
                            Alias::new("inventory_item_id"),
                        )
                        .to(Alias::new("inventory_items"), Alias::new("id"))
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-item-metadata-values-field")
                        .from(
                            Alias::new("inventory_item_metadata_values"),
                            Alias::new("inventory_item_kind_metadata_field_id"),
                        )
                        .to(
                            Alias::new("inventory_item_kind_metadata_fields"),
                            Alias::new("id"),
                        )
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(
            Table::drop()
                .table(Alias::new("inventory_item_metadata_values"))
                .to_owned(),
        )
        .await?;

        m.drop_table(
            Table::drop()
                .table(Alias::new("inventory_item_kind_metadata_fields"))
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}
