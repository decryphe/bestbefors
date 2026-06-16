#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::too_many_lines)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20251020_000001_tables;
mod m20260616_180702_add_inventory_item_kind_metadata;
mod m20260616_183925_add_item_kind_test_standard;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20251020_000001_tables::Migration),
            Box::new(m20260616_180702_add_inventory_item_kind_metadata::Migration),
            Box::new(m20260616_183925_add_item_kind_test_standard::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
