pub use super::_entities::inventory_item_kind_metadata_fields::{ActiveModel, Entity, Model};
use sea_orm::entity::prelude::*;
use std::collections::BTreeSet;
pub type InventoryItemKindMetadataFields = Entity;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl Model {}

impl ActiveModel {}

impl Entity {}

pub fn normalize_metadata_field_names(field_names: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();

    for name in field_names {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            continue;
        }

        let field_name = trimmed.to_string();
        if seen.insert(field_name.to_lowercase()) {
            normalized.push(field_name);
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::normalize_metadata_field_names;

    #[test]
    fn trims_and_deduplicates_metadata_field_names() {
        assert_eq!(
            normalize_metadata_field_names(vec![
                " manufacturer ".to_string(),
                "serialnr".to_string(),
                "Manufacturer".to_string(),
                "".to_string(),
            ]),
            vec!["manufacturer".to_string(), "serialnr".to_string()]
        );
    }
}
