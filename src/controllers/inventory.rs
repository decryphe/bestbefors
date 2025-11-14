use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use std::collections::HashMap;

use crate::models::_entities::{
    checklist_steps::Column as ChecklistStepsColumn,
    executed_checklist_steps::Column as ExecutedChecklistStepsColumn,
    inventory_item_check_steps::Column as InventoryItemCheckStepsColumn,
    inventory_item_checks::Column as InventoryItemChecksColumn,
    inventory_items::Column as InventoryItemsColumn,
};
use crate::{
    exts::{BTreeMapExt, OptionStringExt, StringExt},
    initializers::app_cache::{refresh_item_kinds_cache, AppData},
    models::{
        checklist_steps, checklists, executed_checklist_steps, executed_checklists, expiries,
        intervals, inventory_item_check_steps, inventory_item_checks, inventory_item_kinds,
        inventory_items,
    },
};

#[derive(serde::Serialize)]
struct InventoryItemKindRow {
    kind: inventory_item_kinds::Model,
    default_checklist_name: Option<String>,
    default_interval_code: Option<String>,
    default_expiry_code: Option<String>,
}

#[derive(serde::Serialize)]
struct ItemCheckStepView {
    position: i32,
    name: String,
    description: Option<String>,
    result_code: Option<String>,
    notes: Option<String>,
}

#[derive(serde::Serialize)]
struct ItemCheckView {
    check: inventory_item_checks::Model,
    result_code: Option<String>,
    checked_by: Option<String>,
    steps: Vec<ItemCheckStepView>,
}

#[derive(serde::Serialize)]
struct InventoryListItem {
    #[serde(flatten)]
    item: inventory_items::Model,
    kind_name: Option<String>,
    serial: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct InventoryListQuery {
    q: Option<String>,
}

struct StepResultInput {
    checklist_step_id: i32,
    result_id: i32,
    notes: Option<String>,
}

struct ValidatedCheckPayload {
    steps_template: Vec<checklist_steps::Model>,
    steps: Vec<StepResultInput>,
    notes: Option<String>,
    checked_by: i32,
    result_id: i32,
}

struct ItemFormLookups {
    checklists: Vec<checklists::Model>,
    expiries: Vec<expiries::Model>,
    intervals: Vec<intervals::Model>,
    item_kinds: Vec<inventory_item_kinds::Model>,
}

async fn build_item_form_lookups(ctx: &AppContext) -> Result<ItemFormLookups> {
    let checklists = checklists::Entity::find().all(&ctx.db).await?;
    let expiries = expiries::Entity::find().all(&ctx.db).await?;
    let intervals = intervals::Entity::find().all(&ctx.db).await?;
    let item_kinds = inventory_item_kinds::Entity::find().all(&ctx.db).await?;
    Ok(ItemFormLookups {
        checklists,
        expiries,
        intervals,
        item_kinds,
    })
}

async fn render_inventory_item_form(
    view: &TeraView,
    ctx: &AppContext,
    item: Option<inventory_items::Model>,
    form_action: String,
    form_title: String,
    submit_label: String,
) -> Result<Response> {
    let lookups = build_item_form_lookups(ctx).await?;
    let ItemFormLookups {
        checklists,
        expiries,
        intervals,
        item_kinds,
    } = lookups;
    // let has_item = item.is_some();
    // let item_json = item.map_or(serde_json::Value::Null, |model| {
    //     serde_json::to_value(model).unwrap_or(serde_json::Value::Null)
    // });
    format::render().view(
        view,
        "inventory/add_item.html",
        data!({
            "checklists": checklists,
            "expiries": expiries,
            "intervals": intervals,
            "item_kinds": item_kinds,
            "item": item,
            "form_action": form_action,
            "form_title": form_title,
            "submit_label": submit_label,
            "is_edit": item.is_some(),
        }),
    )
}

#[debug_handler]
pub async fn list(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Query(query): Query<InventoryListQuery>,
) -> Result<Response> {
    let search_term = query.q.and_then(StringExt::clean);
    let search_term_lower = search_term.as_ref().map(|term| term.to_lowercase());
    let item_kinds = ctx.get_item_kinds()?;
    let inventory = inventory_items::Entity::find().all(&ctx.db).await?;

    if let Some(term_lower) = &search_term_lower {
        if let Some(match_item) = inventory.iter().find(|item| {
            item.serial_number
                .as_deref()
                .is_some_and(|serial| serial.to_lowercase() == *term_lower)
        }) {
            return format::redirect(&format!("/inventory/item/{}", match_item.id));
        }
    }

    let inventory = inventory
        .into_iter()
        .filter(|item| {
            if let Some(needle) = search_term_lower.as_deref() {
                let name_match = item.name.to_lowercase().contains(needle);
                let serial_match = item
                    .serial_number
                    .as_deref()
                    .is_some_and(|serial| serial.to_lowercase().contains(needle));
                let kind_match = item_kinds
                    .get(&item.inventory_item_kind_id)
                    .is_some_and(|kind| kind.name.to_lowercase().contains(needle));
                name_match || serial_match || kind_match
            } else {
                true
            }
        })
        .map(|item| InventoryListItem {
            kind_name: item_kinds.get_cloned(&item.inventory_item_kind_id, |kind| &kind.name),
            serial: item.serial_number.clone(),
            item,
        })
        .collect::<Vec<_>>();
    format::render().view(
        &v,
        "inventory/list.html",
        data!({
            "inventory": inventory,
            "inventory_search": search_term,
        }),
    )
}

#[debug_handler]
pub async fn show_item_details(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let Some(item) = inventory_items::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Err(loco_rs::Error::NotFound);
    };

    let item_kinds = ctx.get_item_kinds()?;
    let results = ctx.get_results()?;
    let users = ctx.get_users()?;
    let checklists = ctx.get_checklists()?;
    let intervals = ctx.get_intervals()?;

    let item_kind_name = item_kinds.get_cloned(&item.inventory_item_kind_id, |kind| &kind.name);
    let checklist_name = checklists.get_cloned(&item.checklist_id, |c| &c.name);
    let interval_name = intervals.get_cloned(&item.interval_id, |i| &i.code);

    let checks = inventory_item_checks::Entity::find()
        .filter(InventoryItemChecksColumn::InventoryItemId.eq(item.id))
        .order_by_desc(InventoryItemChecksColumn::CheckedAt)
        .all(&ctx.db)
        .await?;

    let mut rendered_checks = Vec::new();
    for check in checks {
        let executed_steps = executed_checklist_steps::Entity::find()
            .filter(
                ExecutedChecklistStepsColumn::ExecutedChecklistId.eq(check.executed_checklist_id),
            )
            .order_by_asc(ExecutedChecklistStepsColumn::Position)
            .all(&ctx.db)
            .await?;

        let step_results = inventory_item_check_steps::Entity::find()
            .filter(InventoryItemCheckStepsColumn::InventoryItemCheckId.eq(check.id))
            .all(&ctx.db)
            .await?;
        let step_map = step_results
            .into_iter()
            .map(|step| (step.executed_checklist_step_id, step))
            .collect::<HashMap<_, _>>();

        let mut steps_view = Vec::new();
        for executed_step in executed_steps {
            let result = step_map.get(&executed_step.id);
            let result_code = result
                .and_then(|s| results.get(&s.result_id))
                .map(|r| r.code.clone());
            let notes = result.and_then(|s| s.notes.clone());
            steps_view.push(ItemCheckStepView {
                position: executed_step.position,
                name: executed_step.name.clone(),
                description: executed_step.description.clone(),
                result_code,
                notes,
            });
        }

        let rendered = ItemCheckView {
            checked_by: users.get_cloned(&check.checked_by, |user| &user.name),
            result_code: results.get_cloned(&check.result_id, |result| &result.code),
            check,
            steps: steps_view,
        };
        rendered_checks.push(rendered);
    }

    format::render().view(
        &v,
        "inventory/item_details.html",
        data!({
            "item": item,
            "item_kind_name": item_kind_name,
            "checklist_name": checklist_name,
            "interval_name": interval_name,
            "checks": rendered_checks,
        }),
    )
}

#[debug_handler]
pub async fn list_item_kinds(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let item_kinds = ctx.get_item_kinds()?;
    let checklists = ctx.get_checklists()?;
    let intervals = ctx.get_intervals()?;
    let expiries = ctx.get_expiries()?;

    let rows = item_kinds
        .into_values()
        .map(|kind| InventoryItemKindRow {
            default_checklist_name: checklists.get_cloned(&kind.default_checklist_id, |c| &c.name),
            default_interval_code: intervals.get_cloned(&kind.default_interval_id, |i| &i.code),
            default_expiry_code: expiries.get_cloned(&kind.default_expiry_id, |e| &e.code),
            kind,
        })
        .collect::<Vec<_>>();

    format::render().view(
        &v,
        "inventory/item_kinds.html",
        data!({ "item_kinds": rows }),
    )
}

#[debug_handler]
pub async fn add_item(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    render_inventory_item_form(
        &v,
        &ctx,
        None,
        "/inventory/add".to_string(),
        "Add Inventory Item".to_string(),
        "Add Item".to_string(),
    )
    .await
}

#[derive(serde::Deserialize)]
pub struct AddItemPostParams {
    pub name: String,
    pub serial_number: String,
    pub checklist_id: i32,
    pub interval_id: i32,
    pub item_kind_id: i32,
    pub expiry: Option<String>,
}

#[debug_handler]
pub async fn add_item_post(
    State(ctx): State<AppContext>,
    Form(params): Form<AddItemPostParams>,
) -> Result<Response> {
    let AddItemPostParams {
        name,
        serial_number,
        checklist_id,
        interval_id,
        item_kind_id,
        expiry,
    } = params;

    let expiry = if let Some(expiry) = expiry {
        let naive_date = chrono::NaiveDate::parse_from_str(&expiry, "%Y-%m-%d")
            .map_err(|e| loco_rs::Error::BadRequest(e.to_string()))?;
        let naive_datetime = naive_date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            loco_rs::Error::BadRequest("expiry date is outside valid range".to_string())
        })?;
        Some(naive_datetime.and_utc().into())
    } else {
        None
    };

    let serial_number = serial_number.clean();
    let item = crate::models::inventory_items::ActiveModel {
        name: ActiveValue::set(name),
        serial_number: ActiveValue::set(serial_number),
        inventory_item_kind_id: ActiveValue::set(item_kind_id),
        checklist_id: ActiveValue::set(checklist_id),
        interval_id: ActiveValue::set(interval_id),
        expiry: ActiveValue::set(expiry),
        ..Default::default()
    };
    item.insert(&ctx.db).await?;
    format::redirect("/inventory/list")
}

#[debug_handler]
pub async fn edit_item(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let Some(item) = inventory_items::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Err(loco_rs::Error::NotFound);
    };

    render_inventory_item_form(
        &v,
        &ctx,
        Some(item),
        format!("/inventory/item/{id}/edit"),
        "Edit Inventory Item".to_string(),
        "Save Changes".to_string(),
    )
    .await
}

#[debug_handler]
pub async fn edit_item_post(
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    Form(params): Form<AddItemPostParams>,
) -> Result<Response> {
    let Some(existing) = inventory_items::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Err(loco_rs::Error::NotFound);
    };

    let AddItemPostParams {
        name,
        serial_number,
        checklist_id,
        interval_id,
        item_kind_id,
        expiry,
    } = params;

    let expiry = if let Some(expiry) = expiry {
        let naive_date = chrono::NaiveDate::parse_from_str(&expiry, "%Y-%m-%d")
            .map_err(|e| loco_rs::Error::BadRequest(e.to_string()))?;
        let naive_datetime = naive_date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            loco_rs::Error::BadRequest("expiry date is outside valid range".to_string())
        })?;
        Some(naive_datetime.and_utc().into())
    } else {
        None
    };

    let serial_number = serial_number.clean();
    let mut item: inventory_items::ActiveModel = existing.into();
    item.name = ActiveValue::set(name);
    item.serial_number = ActiveValue::set(serial_number);
    item.inventory_item_kind_id = ActiveValue::set(item_kind_id);
    item.checklist_id = ActiveValue::set(checklist_id);
    item.interval_id = ActiveValue::set(interval_id);
    item.expiry = ActiveValue::set(expiry);
    item.update(&ctx.db).await?;

    format::redirect(&format!("/inventory/item/{id}"))
}

#[debug_handler]
pub async fn show_item_check(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let Some(item) = inventory_items::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Err(loco_rs::Error::NotFound);
    };

    let checklists = ctx.get_checklists()?;
    let checklist = checklists
        .get(&item.checklist_id)
        .cloned()
        .ok_or_else(|| loco_rs::Error::NotFound)?;

    let steps = checklist_steps::Entity::find()
        .filter(ChecklistStepsColumn::ChecklistId.eq(checklist.id))
        .order_by_asc(ChecklistStepsColumn::Position)
        .all(&ctx.db)
        .await?;

    let results = ctx.get_results()?.values().cloned().collect::<Vec<_>>();
    let users = ctx.get_users()?.values().cloned().collect::<Vec<_>>();

    format::render().view(
        &v,
        "inventory/check_item.html",
        data!({
            "item": item,
            "checklist": checklist,
            "steps": steps,
            "results": results,
            "users": users,
        }),
    )
}

#[derive(Debug, serde::Deserialize)]
pub struct StepCheckPayload {
    pub checklist_step_id: i32,
    pub result_id: i32,
    pub notes: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PerformCheckPayload {
    pub checked_by: i32,
    pub result_id: i32,
    pub notes: Option<String>,
    #[serde(default)]
    pub steps: Vec<StepCheckPayload>,
}

impl PerformCheckPayload {
    async fn validate(
        self,
        ctx: &AppContext,
        checklist: &checklists::Model,
    ) -> Result<ValidatedCheckPayload> {
        if self.steps.is_empty() {
            return Err(loco_rs::Error::BadRequest(
                "At least one step result must be provided".to_string(),
            ));
        }

        let results = ctx.get_results()?;
        if !results.contains_key(&self.result_id) {
            return Err(loco_rs::Error::BadRequest(
                "Unknown checklist result".to_string(),
            ));
        }

        let users = ctx.get_users()?;
        if !users.contains_key(&self.checked_by) {
            return Err(loco_rs::Error::BadRequest(
                "Unknown user for checklist".to_string(),
            ));
        }

        let steps_template = checklist_steps::Entity::find()
            .filter(ChecklistStepsColumn::ChecklistId.eq(checklist.id))
            .order_by_asc(ChecklistStepsColumn::Position)
            .all(&ctx.db)
            .await?;
        if steps_template.is_empty() {
            return Err(loco_rs::Error::BadRequest(
                "Checklist contains no steps".to_string(),
            ));
        }

        let step_lookup = steps_template
            .iter()
            .map(|step| (step.id, step))
            .collect::<HashMap<_, _>>();

        let notes = self.notes.clean();
        let mut steps = Vec::new();

        for step_payload in self.steps {
            if !step_lookup.contains_key(&step_payload.checklist_step_id) {
                return Err(loco_rs::Error::BadRequest(format!(
                    "Invalid step id {} for checklist",
                    step_payload.checklist_step_id
                )));
            }
            if !results.contains_key(&step_payload.result_id) {
                return Err(loco_rs::Error::BadRequest(format!(
                    "Unknown result {} for step",
                    step_payload.result_id
                )));
            }
            steps.push(StepResultInput {
                checklist_step_id: step_payload.checklist_step_id,
                result_id: step_payload.result_id,
                notes: step_payload.notes.clean(),
            });
        }

        Ok(ValidatedCheckPayload {
            steps_template,
            steps,
            notes,
            checked_by: self.checked_by,
            result_id: self.result_id,
        })
    }
}

#[debug_handler]
pub async fn submit_item_check(
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    Json(payload): Json<PerformCheckPayload>,
) -> Result<Response> {
    let Some(item) = inventory_items::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Err(loco_rs::Error::NotFound);
    };

    let checklists = ctx.get_checklists()?;
    let checklist = checklists
        .get(&item.checklist_id)
        .cloned()
        .ok_or_else(|| loco_rs::Error::InternalServerError)?;

    let validated = payload.validate(&ctx, &checklist).await?;

    let trx = ctx.db.begin().await?;

    let executed_checklist = executed_checklists::ActiveModel {
        name: ActiveValue::set(format!("{} - {}", checklist.name, item.name)),
        description: ActiveValue::set(checklist.description.clone()),
        ..Default::default()
    }
    .insert(&trx)
    .await?;

    let mut executed_step_map = HashMap::new();
    for step in validated.steps_template {
        let exec_step = executed_checklist_steps::ActiveModel {
            executed_checklist_id: ActiveValue::set(executed_checklist.id),
            position: ActiveValue::set(step.position),
            name: ActiveValue::set(step.name.clone()),
            description: ActiveValue::set(step.description.clone()),
            ..Default::default()
        }
        .insert(&trx)
        .await?;
        executed_step_map.insert(step.id, exec_step.id);
    }

    let item_check = inventory_item_checks::ActiveModel {
        finished: ActiveValue::set(true),
        checked_at: ActiveValue::set(Utc::now().into()),
        notes: ActiveValue::set(validated.notes.clone()),
        inventory_item_id: ActiveValue::set(item.id),
        executed_checklist_id: ActiveValue::set(executed_checklist.id),
        checked_by: ActiveValue::set(validated.checked_by),
        result_id: ActiveValue::set(validated.result_id),
        ..Default::default()
    }
    .insert(&trx)
    .await?;

    let mut item_update = item.clone().into_active_model();
    item_update.last_checked_at = ActiveValue::set(Some(item_check.checked_at));
    item_update.update(&trx).await?;

    for step in validated.steps {
        let Some(executed_step_id) = executed_step_map.get(&step.checklist_step_id) else {
            continue;
        };
        inventory_item_check_steps::ActiveModel {
            inventory_item_check_id: ActiveValue::set(item_check.id),
            executed_checklist_step_id: ActiveValue::set(*executed_step_id),
            result_id: ActiveValue::set(step.result_id),
            notes: ActiveValue::set(step.notes.clone()),
            ..Default::default()
        }
        .insert(&trx)
        .await?;
    }

    trx.commit().await?;

    format::json(data!({
        "status": "ok",
        "check_id": item_check.id
    }))
}

#[debug_handler]
pub async fn remove_item(State(ctx): State<AppContext>, Path(id): Path<i32>) -> Result<Response> {
    let check_count = inventory_item_checks::Entity::find()
        .filter(InventoryItemChecksColumn::InventoryItemId.eq(id))
        .count(&ctx.db)
        .await?;

    if check_count > 0 {
        return Err(loco_rs::Error::BadRequest(
            "Inventory item already has completed checks".to_string(),
        ));
    }

    let deleted = inventory_items::Entity::delete_by_id(id)
        .exec(&ctx.db)
        .await?;
    if deleted.rows_affected == 0 {
        return Err(loco_rs::Error::NotFound);
    }

    format::json(data!({ "status": "ok" }))
}

#[debug_handler]
pub async fn add_item_kind_get(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    use crate::models::{checklists, expiries, intervals};
    let checklists = checklists::Entity::find().all(&ctx.db).await?;
    let expiries = expiries::Entity::find().all(&ctx.db).await?;
    let intervals = intervals::Entity::find().all(&ctx.db).await?;
    tracing::info!("loaded checklists: {checklists:?}");
    format::render().view(
        &v,
        "inventory/add_item_kind.html",
        data!({
            "checklists": checklists,
            "expiries": expiries,
            "intervals": intervals,
        }),
    )
}

#[derive(serde::Deserialize)]
pub struct AddItemKindPostParams {
    pub name: String,
    pub default_checklist_id: i32,
    pub default_interval_id: i32,
    pub default_expiry_id: i32,
}

#[debug_handler]
pub async fn add_item_kind_post(
    State(ctx): State<AppContext>,
    Form(params): Form<AddItemKindPostParams>,
) -> Result<Response> {
    let item = crate::models::inventory_item_kinds::ActiveModel {
        name: ActiveValue::set(params.name),
        default_checklist_id: ActiveValue::set(params.default_checklist_id),
        default_interval_id: ActiveValue::set(params.default_interval_id),
        default_expiry_id: ActiveValue::set(params.default_expiry_id),
        ..Default::default()
    };
    item.insert(&ctx.db).await?;
    refresh_item_kinds_cache(&ctx).await?;
    format::redirect("/inventory/item_kinds")
}

#[debug_handler]
pub async fn remove_item_kind(
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let usage_count = inventory_items::Entity::find()
        .filter(InventoryItemsColumn::InventoryItemKindId.eq(id))
        .count(&ctx.db)
        .await?;

    if usage_count > 0 {
        return Err(loco_rs::Error::BadRequest(
            "Item kind is in use by inventory items".to_string(),
        ));
    }

    let deleted = inventory_item_kinds::Entity::delete_by_id(id)
        .exec(&ctx.db)
        .await?;
    if deleted.rows_affected == 0 {
        return Err(loco_rs::Error::NotFound);
    }

    refresh_item_kinds_cache(&ctx).await?;

    format::json(data!({ "status": "ok" }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("inventory")
        .add("/add", get(add_item))
        .add("/add", post(add_item_post))
        .add("/item/{id}", get(show_item_details))
        .add("/item/{id}/edit", get(edit_item))
        .add("/item/{id}/check", get(show_item_check))
        .add("/item/{id}/check", post(submit_item_check))
        .add("/item/{id}/edit", post(edit_item_post))
        .add("/item/{id}", delete(remove_item))
        .add("/add_item_kind", get(add_item_kind_get))
        .add("/add_item_kind", post(add_item_kind_post))
        .add("/item_kinds", get(list_item_kinds))
        .add("/item_kinds/{id}", delete(remove_item_kind))
        .add("/list", get(list))
}
