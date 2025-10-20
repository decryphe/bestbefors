#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use loco_rs::prelude::*;

use crate::models::*;

#[derive(serde::Serialize)]
struct HomeEntryCheck {
    check: inventory_item_checks::Model,
    checked_by: String,
    result_code: String,
    checklist_name: String,
    checklist_description: Option<String>,
}

#[derive(serde::Serialize)]
struct HomeEntry {
    item: inventory_items::Model,
    item_checks: Vec<HomeEntryCheck>,
    checklist_name: String,
    checklist_description: Option<String>,
    item_kind_name: String,
    interval: intervals::Model,
    next_expiry: DateTimeWithTimeZone,
}

pub async fn home(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    use crate::initializers::app_cache::AppData;
    let checklists = ctx.get_checklists()?;
    let intervals = ctx.get_intervals()?;
    let item_kinds = ctx.get_item_kinds()?;
    let results = ctx.get_results()?;
    let users = ctx.get_users()?;

    let items = inventory_items::Entity::find()
        .find_with_related(inventory_item_checks::Entity)
        .all(&ctx.db)
        .await?;

    let mut items: Vec<HomeEntry> = items
        .into_iter()
        .map(|(item, checks)| {
            let checklist = checklists.get(&item.checklist_id)?;
            let item_kind = item_kinds.get(&item.inventory_item_kind_id)?;
            let interval = intervals.get(&item.interval_id)?;
            let mut next_expiry =
                interval.next_interval_expiry(&item.created_at, &item.last_checked_at);
            if let Some(expiry) = item.expiry {
                if expiry < next_expiry {
                    next_expiry = expiry;
                }
            }

            Some(HomeEntry {
                item,
                item_checks: checks
                    .into_iter()
                    .map(|check| {
                        let checklist = checklists.get(&check.executed_checklist_id)?;
                        let user = users.get(&check.checked_by)?;
                        let result = results.get(&check.result_id)?;

                        Some(HomeEntryCheck {
                            check,
                            checked_by: user.name.clone(),
                            result_code: result.code.clone(),
                            checklist_name: checklist.name.clone(),
                            checklist_description: checklist.description.clone(),
                        })
                    })
                    .filter_map(|c| c)
                    .collect(),
                checklist_name: checklist.name.clone(),
                checklist_description: checklist.description.clone(),
                item_kind_name: item_kind.name.clone(),
                interval: interval.clone(),
                next_expiry,
            })
        })
        .filter_map(|c| c)
        .collect();
    items.sort_unstable_by_key(|i| i.next_expiry);

    format::render().view(&v, "home/home.html", data!({ "items": items }))
}

#[debug_handler]
pub async fn manage(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "home/manage.html", data!({}))
}

pub fn routes() -> Routes {
    Routes::new().prefix("/").add("", get(home)).add("manage", get(manage))
}
