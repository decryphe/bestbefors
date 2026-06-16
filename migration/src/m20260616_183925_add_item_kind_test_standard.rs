use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("inventory_item_kinds"))
                .add_column(string(Alias::new("test_standard")).not_null().default(""))
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("inventory_item_kinds"))
                .drop_column(Alias::new("test_standard"))
                .to_owned(),
        )
        .await?;

        Ok(())
    }
}
