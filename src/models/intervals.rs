pub use super::_entities::intervals::{ActiveModel, Entity, Model};
use chrono::Datelike;
use sea_orm::entity::prelude::*;
pub type Intervals = Entity;

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

// implement your read-oriented logic here
impl Model {
    #[must_use]
    pub fn next_interval_expiry(
        &self,
        created_at: &DateTimeWithTimeZone,
        last_checked_at: &Option<DateTimeWithTimeZone>,
    ) -> DateTimeWithTimeZone {
        let starting_point = match last_checked_at {
            Some(last_checked_at) => last_checked_at,
            None => created_at,
        };

        let end_point = match self.sqlite_modifier.as_str() {
            "days" => {
                let days = chrono::Days::new(self.sqlite_num_of_modifier.unsigned_abs().into());
                if self.sqlite_num_of_modifier > 0 {
                    starting_point.checked_add_days(days)
                } else {
                    starting_point.checked_sub_days(days)
                }
            }
            "months" => {
                let months = chrono::Months::new(self.sqlite_num_of_modifier.unsigned_abs());
                if self.sqlite_num_of_modifier > 0 {
                    starting_point.checked_add_months(months)
                } else {
                    starting_point.checked_sub_months(months)
                }
            }
            "years" => {
                starting_point.with_year(starting_point.year() + self.sqlite_num_of_modifier)
            }
            _ => None,
        };
        end_point.unwrap_or_default()
    }
}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
