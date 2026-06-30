use axum::{
    http::{header, HeaderValue},
    response::IntoResponse,
};
use axum_extra::extract::Form as HtmlForm;
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
    inventory_item_kind_metadata_fields::Column as InventoryItemKindMetadataFieldsColumn,
    inventory_item_metadata_values::Column as InventoryItemMetadataValuesColumn,
    inventory_items::Column as InventoryItemsColumn,
};
use crate::{
    exts::{BTreeMapExt, OptionStringExt, StringExt},
    initializers::app_cache::{refresh_item_kinds_cache, AppData},
    models::{
        checklist_steps, checklists, executed_checklist_steps, executed_checklists, expiries,
        intervals, inventory_item_check_steps, inventory_item_checks,
        inventory_item_kind_metadata_fields, inventory_item_kinds, inventory_item_metadata_values,
        inventory_items,
    },
    reports::single_item_history::{
        self, ReportCheck, ReportField, ReportItem, ReportStep, SingleItemHistoryReport,
    },
};

#[derive(serde::Serialize)]
struct InventoryItemKindRow {
    kind: inventory_item_kinds::Model,
    default_checklist_name: Option<String>,
    default_interval_code: Option<String>,
    default_expiry_code: Option<String>,
    metadata_fields: Vec<String>,
}

#[derive(serde::Serialize)]
struct InventoryItemKindDetailView {
    kind: inventory_item_kinds::Model,
    default_checklist_name: Option<String>,
    default_interval_code: Option<String>,
    default_expiry_code: Option<String>,
    metadata_fields: Vec<String>,
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
    metadata_cells: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
struct InventoryMetadataColumn {
    key: String,
    name: String,
}

#[derive(Clone, serde::Serialize)]
struct MetadataFieldDefinition {
    id: i32,
    name: String,
}

#[derive(serde::Serialize)]
struct ItemMetadataValueView {
    name: String,
    value: String,
}

struct ItemDetailsData {
    item: inventory_items::Model,
    item_kind_name: Option<String>,
    checklist_name: Option<String>,
    interval_name: Option<String>,
    metadata: Vec<ItemMetadataValueView>,
    checks: Vec<ItemCheckView>,
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

struct ItemKindFormLookups {
    checklists: Vec<checklists::Model>,
    expiries: Vec<expiries::Model>,
    intervals: Vec<intervals::Model>,
}

#[derive(Clone, serde::Serialize)]
struct MetadataFieldFormRow {
    name: String,
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
) -> Result<Response> {
    let lookups = build_item_form_lookups(ctx).await?;
    let ItemFormLookups {
        checklists,
        expiries,
        intervals,
        item_kinds,
    } = lookups;
    let item_kind_ids = item_kinds.iter().map(|kind| kind.id).collect::<Vec<_>>();
    let metadata_fields_by_kind_id =
        load_metadata_fields_by_kind_ids(&ctx.db, item_kind_ids).await?;
    let metadata_field_definitions_by_kind_id = metadata_fields_by_kind_id
        .into_iter()
        .map(|(kind_id, fields)| {
            (
                kind_id,
                fields
                    .into_iter()
                    .map(|field| MetadataFieldDefinition {
                        id: field.id,
                        name: field.name,
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    let has_item = item.is_some();
    let item_metadata_values = if let Some(model) = item.as_ref() {
        let values_by_item_id =
            load_metadata_value_maps_by_field_id_for_items(&ctx.db, &[model.id]).await?;
        values_by_item_id
            .get(&model.id)
            .cloned()
            .unwrap_or_default()
    } else {
        HashMap::new()
    };
    let item_json = item.map_or(serde_json::Value::Null, |model| {
        serde_json::to_value(model).unwrap_or(serde_json::Value::Null)
    });
    format::render().view(
        view,
        "inventory/add_item.html",
        data!({
            "checklists": checklists,
            "expiries": expiries,
            "intervals": intervals,
            "item_kinds": item_kinds,
            "item_kind_metadata_fields": metadata_field_definitions_by_kind_id,
            "item": item_json,
            "item_metadata_values": item_metadata_values,
            "form_action": form_action,
            "is_edit": has_item,
        }),
    )
}

async fn build_item_kind_form_lookups(ctx: &AppContext) -> Result<ItemKindFormLookups> {
    let checklists = checklists::Entity::find().all(&ctx.db).await?;
    let expiries = expiries::Entity::find().all(&ctx.db).await?;
    let intervals = intervals::Entity::find().all(&ctx.db).await?;
    Ok(ItemKindFormLookups {
        checklists,
        expiries,
        intervals,
    })
}

async fn load_metadata_field_names_by_kind_ids<C>(
    db: &C,
    kind_ids: Vec<i32>,
) -> Result<HashMap<i32, Vec<String>>>
where
    C: ConnectionTrait,
{
    if kind_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let metadata_fields = inventory_item_kind_metadata_fields::Entity::find()
        .filter(InventoryItemKindMetadataFieldsColumn::InventoryItemKindId.is_in(kind_ids))
        .order_by_asc(InventoryItemKindMetadataFieldsColumn::Position)
        .all(db)
        .await?;

    let mut by_kind_id = HashMap::<i32, Vec<String>>::new();
    for field in metadata_fields {
        by_kind_id
            .entry(field.inventory_item_kind_id)
            .or_default()
            .push(field.name);
    }

    Ok(by_kind_id)
}

async fn load_metadata_fields_by_kind_ids<C>(
    db: &C,
    kind_ids: Vec<i32>,
) -> Result<HashMap<i32, Vec<inventory_item_kind_metadata_fields::Model>>>
where
    C: ConnectionTrait,
{
    if kind_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let metadata_fields = inventory_item_kind_metadata_fields::Entity::find()
        .filter(InventoryItemKindMetadataFieldsColumn::InventoryItemKindId.is_in(kind_ids))
        .order_by_asc(InventoryItemKindMetadataFieldsColumn::Position)
        .all(db)
        .await?;

    let mut by_kind_id = HashMap::<i32, Vec<inventory_item_kind_metadata_fields::Model>>::new();
    for field in metadata_fields {
        by_kind_id
            .entry(field.inventory_item_kind_id)
            .or_default()
            .push(field);
    }

    Ok(by_kind_id)
}

async fn load_metadata_field_form_rows<C>(
    db: &C,
    item_kind_id: i32,
) -> Result<Vec<MetadataFieldFormRow>>
where
    C: ConnectionTrait,
{
    Ok(inventory_item_kind_metadata_fields::Entity::find()
        .filter(InventoryItemKindMetadataFieldsColumn::InventoryItemKindId.eq(item_kind_id))
        .order_by_asc(InventoryItemKindMetadataFieldsColumn::Position)
        .all(db)
        .await?
        .into_iter()
        .map(|field| MetadataFieldFormRow { name: field.name })
        .collect())
}

async fn load_metadata_value_maps_for_items<C>(
    db: &C,
    item_ids: &[i32],
) -> Result<HashMap<i32, HashMap<String, String>>>
where
    C: ConnectionTrait,
{
    if item_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let values = inventory_item_metadata_values::Entity::find()
        .filter(InventoryItemMetadataValuesColumn::InventoryItemId.is_in(item_ids.to_vec()))
        .all(db)
        .await?;

    if values.is_empty() {
        return Ok(HashMap::new());
    }

    let field_ids = values
        .iter()
        .map(|value| value.inventory_item_kind_metadata_field_id)
        .collect::<Vec<_>>();
    let fields = inventory_item_kind_metadata_fields::Entity::find()
        .filter(InventoryItemKindMetadataFieldsColumn::Id.is_in(field_ids))
        .all(db)
        .await?;
    let field_names_by_id = fields
        .into_iter()
        .map(|field| (field.id, field.name))
        .collect::<HashMap<_, _>>();

    let mut values_by_item_id = HashMap::<i32, HashMap<String, String>>::new();
    for value in values {
        let Some(field_name) = field_names_by_id.get(&value.inventory_item_kind_metadata_field_id)
        else {
            continue;
        };

        values_by_item_id
            .entry(value.inventory_item_id)
            .or_default()
            .insert(field_name.clone(), value.value);
    }

    Ok(values_by_item_id)
}

async fn load_metadata_value_maps_by_field_id_for_items<C>(
    db: &C,
    item_ids: &[i32],
) -> Result<HashMap<i32, HashMap<i32, String>>>
where
    C: ConnectionTrait,
{
    if item_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let values = inventory_item_metadata_values::Entity::find()
        .filter(InventoryItemMetadataValuesColumn::InventoryItemId.is_in(item_ids.to_vec()))
        .all(db)
        .await?;

    let mut values_by_item_id = HashMap::<i32, HashMap<i32, String>>::new();
    for value in values {
        values_by_item_id
            .entry(value.inventory_item_id)
            .or_default()
            .insert(value.inventory_item_kind_metadata_field_id, value.value);
    }

    Ok(values_by_item_id)
}

fn metadata_column_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn build_inventory_metadata_columns(
    items: &[inventory_items::Model],
    metadata_fields_by_kind_id: &HashMap<i32, Vec<inventory_item_kind_metadata_fields::Model>>,
) -> Vec<InventoryMetadataColumn> {
    let mut columns = Vec::new();
    let mut seen = HashMap::<String, usize>::new();

    for item in items {
        let Some(fields) = metadata_fields_by_kind_id.get(&item.inventory_item_kind_id) else {
            continue;
        };

        for field in fields {
            let key = metadata_column_key(&field.name);
            if key.is_empty() || seen.contains_key(&key) {
                continue;
            }

            seen.insert(key.clone(), columns.len());
            columns.push(InventoryMetadataColumn {
                key,
                name: field.name.clone(),
            });
        }
    }

    columns
}

fn build_inventory_metadata_values_by_column_key(
    metadata_fields: &[inventory_item_kind_metadata_fields::Model],
    metadata_values_by_field_id: &HashMap<i32, String>,
) -> HashMap<String, String> {
    let mut values_by_column_key = HashMap::<String, Vec<String>>::new();

    for field in metadata_fields {
        let key = metadata_column_key(&field.name);
        if key.is_empty() {
            continue;
        }

        let Some(value) = metadata_values_by_field_id.get(&field.id) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        let column_values = values_by_column_key.entry(key).or_default();
        if !column_values.iter().any(|existing| existing == value) {
            column_values.push(value.to_string());
        }
    }

    values_by_column_key
        .into_iter()
        .map(|(key, values)| (key, values.join(", ")))
        .collect()
}

fn metadata_values_from_form(
    metadata_field_ids: Option<Vec<i32>>,
    metadata_values: Option<Vec<String>>,
) -> HashMap<i32, String> {
    metadata_field_ids
        .unwrap_or_default()
        .into_iter()
        .zip(metadata_values.unwrap_or_default())
        .collect()
}

async fn replace_metadata_fields_for_kind<C>(
    db: &C,
    item_kind_id: i32,
    field_names: &[String],
) -> Result<Vec<inventory_item_kind_metadata_fields::Model>>
where
    C: ConnectionTrait,
{
    inventory_item_kind_metadata_fields::Entity::delete_many()
        .filter(InventoryItemKindMetadataFieldsColumn::InventoryItemKindId.eq(item_kind_id))
        .exec(db)
        .await?;

    let mut created_fields = Vec::new();
    for (position, field_name) in field_names.iter().enumerate() {
        let now = Utc::now();
        let created = inventory_item_kind_metadata_fields::ActiveModel {
            created_at: ActiveValue::set(now.into()),
            inventory_item_kind_id: ActiveValue::set(item_kind_id),
            name: ActiveValue::set(field_name.clone()),
            position: ActiveValue::set(position as i32),
            updated_at: ActiveValue::set(now.into()),
            ..Default::default()
        }
        .insert(db)
        .await?;
        created_fields.push(created);
    }

    Ok(created_fields)
}

async fn replace_metadata_values_for_item<C>(
    db: &C,
    item_id: i32,
    metadata_fields: &[inventory_item_kind_metadata_fields::Model],
    existing_values_by_name: &HashMap<String, String>,
) -> Result<()>
where
    C: ConnectionTrait,
{
    inventory_item_metadata_values::Entity::delete_many()
        .filter(InventoryItemMetadataValuesColumn::InventoryItemId.eq(item_id))
        .exec(db)
        .await?;

    for field in metadata_fields {
        let now = Utc::now();
        inventory_item_metadata_values::ActiveModel {
            created_at: ActiveValue::set(now.into()),
            inventory_item_id: ActiveValue::set(item_id),
            inventory_item_kind_metadata_field_id: ActiveValue::set(field.id),
            updated_at: ActiveValue::set(now.into()),
            value: ActiveValue::set(
                existing_values_by_name
                    .get(&field.name)
                    .cloned()
                    .unwrap_or_default(),
            ),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn replace_metadata_values_for_item_by_field_id<C>(
    db: &C,
    item_id: i32,
    metadata_fields: &[inventory_item_kind_metadata_fields::Model],
    values_by_field_id: &HashMap<i32, String>,
) -> Result<()>
where
    C: ConnectionTrait,
{
    inventory_item_metadata_values::Entity::delete_many()
        .filter(InventoryItemMetadataValuesColumn::InventoryItemId.eq(item_id))
        .exec(db)
        .await?;

    for field in metadata_fields {
        let now = Utc::now();
        inventory_item_metadata_values::ActiveModel {
            created_at: ActiveValue::set(now.into()),
            inventory_item_id: ActiveValue::set(item_id),
            inventory_item_kind_metadata_field_id: ActiveValue::set(field.id),
            updated_at: ActiveValue::set(now.into()),
            value: ActiveValue::set(
                values_by_field_id
                    .get(&field.id)
                    .cloned()
                    .unwrap_or_default(),
            ),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn rebuild_metadata_values_for_kind_items<C>(
    db: &C,
    item_kind_id: i32,
    item_ids: &[i32],
    existing_values_by_item_id: HashMap<i32, HashMap<String, String>>,
) -> Result<()>
where
    C: ConnectionTrait,
{
    let metadata_fields = inventory_item_kind_metadata_fields::Entity::find()
        .filter(InventoryItemKindMetadataFieldsColumn::InventoryItemKindId.eq(item_kind_id))
        .order_by_asc(InventoryItemKindMetadataFieldsColumn::Position)
        .all(db)
        .await?;

    for item_id in item_ids {
        let existing_values = existing_values_by_item_id
            .get(item_id)
            .cloned()
            .unwrap_or_default();
        replace_metadata_values_for_item(db, *item_id, &metadata_fields, &existing_values).await?;
    }

    Ok(())
}

async fn render_item_kind_form(
    view: &TeraView,
    ctx: &AppContext,
    item_kind: Option<inventory_item_kinds::Model>,
    form_action: String,
) -> Result<Response> {
    let lookups = build_item_kind_form_lookups(ctx).await?;
    let metadata_fields = match item_kind.as_ref() {
        Some(item_kind) => load_metadata_field_form_rows(&ctx.db, item_kind.id).await?,
        None => Vec::new(),
    };
    let has_item_kind = item_kind.is_some();

    format::render().view(
        view,
        "inventory/add_item_kind.html",
        data!({
            "checklists": lookups.checklists,
            "expiries": lookups.expiries,
            "intervals": lookups.intervals,
            "item_kind": item_kind,
            "metadata_fields": metadata_fields,
            "form_action": form_action,
            "is_edit": has_item_kind,
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
        .collect::<Vec<_>>();

    let item_ids = inventory.iter().map(|item| item.id).collect::<Vec<_>>();
    let kind_ids = inventory
        .iter()
        .map(|item| item.inventory_item_kind_id)
        .collect::<Vec<_>>();
    let metadata_fields_by_kind_id = load_metadata_fields_by_kind_ids(&ctx.db, kind_ids).await?;
    let metadata_values_by_item_id =
        load_metadata_value_maps_by_field_id_for_items(&ctx.db, &item_ids).await?;
    let metadata_columns =
        build_inventory_metadata_columns(&inventory, &metadata_fields_by_kind_id);
    let empty_metadata_values = HashMap::new();

    let inventory = inventory
        .into_iter()
        .map(|item| {
            let item_metadata_values = metadata_values_by_item_id
                .get(&item.id)
                .unwrap_or(&empty_metadata_values);
            let metadata_values_by_column_key = metadata_fields_by_kind_id
                .get(&item.inventory_item_kind_id)
                .map(|metadata_fields| {
                    build_inventory_metadata_values_by_column_key(
                        metadata_fields,
                        item_metadata_values,
                    )
                })
                .unwrap_or_default();

            InventoryListItem {
                kind_name: item_kinds.get_cloned(&item.inventory_item_kind_id, |kind| &kind.name),
                serial: item.serial_number.clone(),
                metadata_cells: metadata_columns
                    .iter()
                    .map(|column| {
                        metadata_values_by_column_key
                            .get(&column.key)
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect(),
                item,
            }
        })
        .collect::<Vec<_>>();

    format::render().view(
        &v,
        "inventory/list.html",
        data!({
            "inventory": inventory,
            "inventory_search": search_term,
            "metadata_columns": metadata_columns,
        }),
    )
}

#[debug_handler]
pub async fn show_item_details(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let details = load_item_details_data(&ctx, id).await?;
    let ItemDetailsData {
        item,
        item_kind_name,
        checklist_name,
        interval_name,
        metadata,
        checks,
    } = details;

    format::render().view(
        &v,
        "inventory/item_details.html",
        data!({
            "item": item,
            "item_kind_name": item_kind_name,
            "checklist_name": checklist_name,
            "interval_name": interval_name,
            "metadata": metadata,
            "checks": checks,
        }),
    )
}

#[debug_handler]
pub async fn download_item_report(
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let details = load_item_details_data(&ctx, id).await?;
    let report = build_single_item_history_report(details);
    let pdf = single_item_history::render_pdf(&report)?;
    let filename = format!("inventory-item-{id}-history.pdf");
    let content_disposition =
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|_| loco_rs::Error::InternalServerError)?;

    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            ),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        pdf,
    )
        .into_response())
}

async fn load_item_details_data(ctx: &AppContext, id: i32) -> Result<ItemDetailsData> {
    let Some(item) = inventory_items::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Err(loco_rs::Error::NotFound);
    };

    let item_kinds = ctx.get_item_kinds()?;
    let results = ctx.get_results()?;
    let users = ctx.get_users()?;
    let checklists = ctx.get_checklists()?;
    let intervals = ctx.get_intervals()?;
    let metadata_fields_by_kind_id =
        load_metadata_fields_by_kind_ids(&ctx.db, vec![item.inventory_item_kind_id]).await?;
    let metadata_values_by_item_id =
        load_metadata_value_maps_by_field_id_for_items(&ctx.db, &[item.id]).await?;

    let item_kind_name = item_kinds.get_cloned(&item.inventory_item_kind_id, |kind| &kind.name);
    let checklist_name = checklists.get_cloned(&item.checklist_id, |c| &c.name);
    let interval_name = intervals.get_cloned(&item.interval_id, |i| &i.code);
    let metadata_values = metadata_values_by_item_id
        .get(&item.id)
        .cloned()
        .unwrap_or_default();
    let metadata = metadata_fields_by_kind_id
        .get(&item.inventory_item_kind_id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|field| ItemMetadataValueView {
            name: field.name,
            value: metadata_values.get(&field.id).cloned().unwrap_or_default(),
        })
        .collect::<Vec<_>>();

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

    Ok(ItemDetailsData {
        item,
        item_kind_name,
        checklist_name,
        interval_name,
        metadata,
        checks: rendered_checks,
    })
}

fn build_single_item_history_report(details: ItemDetailsData) -> SingleItemHistoryReport {
    let ItemDetailsData {
        item,
        item_kind_name,
        checklist_name,
        interval_name,
        metadata,
        checks,
    } = details;

    SingleItemHistoryReport {
        title: "Inventory Item Check History".to_string(),
        generated_at: single_item_history::format_generated_at(Utc::now()),
        report_id: format!("ITEM-{}", item.id),
        item: ReportItem {
            name: item.name,
            serial_number: item.serial_number,
            item_kind: item_kind_name,
            checklist: checklist_name,
            interval: interval_name.map(|code| single_item_history::humanize_code(&code)),
            created_at: single_item_history::format_timestamp(item.created_at),
            updated_at: single_item_history::format_timestamp(item.updated_at),
            last_checked_at: item
                .last_checked_at
                .map(single_item_history::format_timestamp),
            expiry: item.expiry.map(single_item_history::format_timestamp),
        },
        metadata: metadata
            .into_iter()
            .map(|entry| ReportField {
                label: entry.name,
                value: entry.value.clean(),
            })
            .collect(),
        checks: checks
            .into_iter()
            .map(|check| ReportCheck {
                checked_at: single_item_history::format_timestamp(check.check.checked_at),
                checked_by: check.checked_by,
                overall_result: check
                    .result_code
                    .map(|code| single_item_history::humanize_code(&code)),
                notes: check.check.notes,
                steps: check
                    .steps
                    .into_iter()
                    .map(|step| ReportStep {
                        position: step.position,
                        name: step.name,
                        result: step
                            .result_code
                            .map(|code| single_item_history::humanize_code(&code)),
                        notes: step.notes,
                    })
                    .collect(),
            })
            .collect(),
    }
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
    let kind_ids = item_kinds.keys().copied().collect::<Vec<_>>();
    let metadata_field_names_by_kind_id =
        load_metadata_field_names_by_kind_ids(&ctx.db, kind_ids).await?;

    let rows = item_kinds
        .into_values()
        .map(|kind| InventoryItemKindRow {
            default_checklist_name: checklists.get_cloned(&kind.default_checklist_id, |c| &c.name),
            default_interval_code: intervals.get_cloned(&kind.default_interval_id, |i| &i.code),
            default_expiry_code: expiries.get_cloned(&kind.default_expiry_id, |e| &e.code),
            metadata_fields: metadata_field_names_by_kind_id
                .get(&kind.id)
                .cloned()
                .unwrap_or_default(),
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
pub async fn show_item_kind(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let Some(kind) = inventory_item_kinds::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
    else {
        return Err(loco_rs::Error::NotFound);
    };

    let checklists = ctx.get_checklists()?;
    let intervals = ctx.get_intervals()?;
    let expiries = ctx.get_expiries()?;
    let metadata_fields = load_metadata_field_names_by_kind_ids(&ctx.db, vec![id]).await?;

    let detail = InventoryItemKindDetailView {
        default_checklist_name: checklists.get_cloned(&kind.default_checklist_id, |c| &c.name),
        default_interval_code: intervals.get_cloned(&kind.default_interval_id, |i| &i.code),
        default_expiry_code: expiries.get_cloned(&kind.default_expiry_id, |e| &e.code),
        metadata_fields: metadata_fields.get(&id).cloned().unwrap_or_default(),
        kind,
    };

    format::render().view(
        &v,
        "inventory/item_kind_details.html",
        data!({ "item_kind": detail }),
    )
}

#[debug_handler]
pub async fn add_item(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    render_inventory_item_form(&v, &ctx, None, "/inventory/add".to_string()).await
}

#[derive(serde::Deserialize)]
pub struct AddItemPostParams {
    pub name: String,
    pub serial_number: String,
    pub checklist_id: i32,
    pub interval_id: i32,
    pub item_kind_id: i32,
    pub expiry: Option<String>,
    #[serde(default, alias = "metadata_field_ids[]")]
    pub metadata_field_ids: Option<Vec<i32>>,
    #[serde(default, alias = "metadata_values[]")]
    pub metadata_values: Option<Vec<String>>,
}

#[debug_handler]
pub async fn add_item_post(
    State(ctx): State<AppContext>,
    HtmlForm(params): HtmlForm<AddItemPostParams>,
) -> Result<Response> {
    let AddItemPostParams {
        name,
        serial_number,
        checklist_id,
        interval_id,
        item_kind_id,
        expiry,
        metadata_field_ids,
        metadata_values,
    } = params;
    if !ctx.get_item_kinds()?.contains_key(&item_kind_id) {
        return Err(loco_rs::Error::BadRequest("Unknown item kind".to_string()));
    }
    let submitted_metadata_values = metadata_values_from_form(metadata_field_ids, metadata_values);

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
    let created_item = item.insert(&ctx.db).await?;
    let metadata_fields = load_metadata_fields_by_kind_ids(&ctx.db, vec![item_kind_id]).await?;
    replace_metadata_values_for_item_by_field_id(
        &ctx.db,
        created_item.id,
        &metadata_fields
            .get(&item_kind_id)
            .cloned()
            .unwrap_or_default(),
        &submitted_metadata_values,
    )
    .await?;
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

    render_inventory_item_form(&v, &ctx, Some(item), format!("/inventory/item/{id}/edit")).await
}

#[debug_handler]
pub async fn edit_item_post(
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    HtmlForm(params): HtmlForm<AddItemPostParams>,
) -> Result<Response> {
    let Some(existing) = inventory_items::Entity::find_by_id(id).one(&ctx.db).await? else {
        return Err(loco_rs::Error::NotFound);
    };
    let existing_item_id = existing.id;

    let AddItemPostParams {
        name,
        serial_number,
        checklist_id,
        interval_id,
        item_kind_id,
        expiry,
        metadata_field_ids,
        metadata_values,
    } = params;
    if !ctx.get_item_kinds()?.contains_key(&item_kind_id) {
        return Err(loco_rs::Error::BadRequest("Unknown item kind".to_string()));
    }
    let submitted_metadata_values = metadata_values_from_form(metadata_field_ids, metadata_values);

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
    let metadata_fields = load_metadata_fields_by_kind_ids(&ctx.db, vec![item_kind_id]).await?;
    replace_metadata_values_for_item_by_field_id(
        &ctx.db,
        existing_item_id,
        &metadata_fields
            .get(&item_kind_id)
            .cloned()
            .unwrap_or_default(),
        &submitted_metadata_values,
    )
    .await?;

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
pub async fn add_item_kind_new(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    render_item_kind_form(&v, &ctx, None, "/inventory/item_kinds/new".to_string()).await
}

pub struct AddItemKindPostParams {
    pub name: String,
    pub test_standard: String,
    pub default_checklist_id: i32,
    pub default_interval_id: i32,
    pub default_expiry_id: i32,
    pub metadata_field_names: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum OneOrManyStrings {
    One(String),
    Many(Vec<String>),
}

impl<'de> serde::Deserialize<'de> for AddItemKindPostParams {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AddItemKindPostParamsVisitor;

        impl<'de> serde::de::Visitor<'de> for AddItemKindPostParamsVisitor {
            type Value = AddItemKindPostParams;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("application/x-www-form-urlencoded item kind params")
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut name = None;
                let mut test_standard = None;
                let mut default_checklist_id = None;
                let mut default_interval_id = None;
                let mut default_expiry_id = None;
                let mut metadata_field_names = Vec::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => {
                            if name.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        "test_standard" => {
                            if test_standard.is_some() {
                                return Err(serde::de::Error::duplicate_field("test_standard"));
                            }
                            test_standard = Some(map.next_value()?);
                        }
                        "default_checklist_id" => {
                            if default_checklist_id.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "default_checklist_id",
                                ));
                            }
                            default_checklist_id = Some(map.next_value()?);
                        }
                        "default_interval_id" => {
                            if default_interval_id.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "default_interval_id",
                                ));
                            }
                            default_interval_id = Some(map.next_value()?);
                        }
                        "default_expiry_id" => {
                            if default_expiry_id.is_some() {
                                return Err(serde::de::Error::duplicate_field("default_expiry_id"));
                            }
                            default_expiry_id = Some(map.next_value()?);
                        }
                        "metadata_field_names" | "metadata_field_names[]" => {
                            match map.next_value::<OneOrManyStrings>()? {
                                OneOrManyStrings::One(value) => metadata_field_names.push(value),
                                OneOrManyStrings::Many(values) => {
                                    metadata_field_names.extend(values)
                                }
                            }
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                Ok(AddItemKindPostParams {
                    name: name.ok_or_else(|| serde::de::Error::missing_field("name"))?,
                    test_standard: test_standard
                        .ok_or_else(|| serde::de::Error::missing_field("test_standard"))?,
                    default_checklist_id: default_checklist_id
                        .ok_or_else(|| serde::de::Error::missing_field("default_checklist_id"))?,
                    default_interval_id: default_interval_id
                        .ok_or_else(|| serde::de::Error::missing_field("default_interval_id"))?,
                    default_expiry_id: default_expiry_id
                        .ok_or_else(|| serde::de::Error::missing_field("default_expiry_id"))?,
                    metadata_field_names: if metadata_field_names.is_empty() {
                        None
                    } else {
                        Some(metadata_field_names)
                    },
                })
            }
        }

        deserializer.deserialize_map(AddItemKindPostParamsVisitor)
    }
}

#[debug_handler]
pub async fn add_item_kind_new_post(
    State(ctx): State<AppContext>,
    HtmlForm(params): HtmlForm<AddItemKindPostParams>,
) -> Result<Response> {
    let AddItemKindPostParams {
        name,
        test_standard,
        default_checklist_id,
        default_interval_id,
        default_expiry_id,
        metadata_field_names,
    } = params;
    let metadata_field_names = inventory_item_kind_metadata_fields::normalize_metadata_field_names(
        metadata_field_names.unwrap_or_default(),
    );
    let trx = ctx.db.begin().await?;
    let item = crate::models::inventory_item_kinds::ActiveModel {
        name: ActiveValue::set(name.trim().to_string()),
        test_standard: ActiveValue::set(test_standard.trim().to_string()),
        default_checklist_id: ActiveValue::set(default_checklist_id),
        default_interval_id: ActiveValue::set(default_interval_id),
        default_expiry_id: ActiveValue::set(default_expiry_id),
        ..Default::default()
    };
    let created_kind = item.insert(&trx).await?;
    replace_metadata_fields_for_kind(&trx, created_kind.id, &metadata_field_names).await?;
    trx.commit().await?;
    refresh_item_kinds_cache(&ctx).await?;
    format::redirect("/inventory/item_kinds")
}

#[debug_handler]
pub async fn edit_item_kind(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let Some(item_kind) = inventory_item_kinds::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
    else {
        return Err(loco_rs::Error::NotFound);
    };

    render_item_kind_form(
        &v,
        &ctx,
        Some(item_kind),
        format!("/inventory/item_kinds/{id}/edit"),
    )
    .await
}

#[debug_handler]
pub async fn edit_item_kind_post(
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
    HtmlForm(params): HtmlForm<AddItemKindPostParams>,
) -> Result<Response> {
    let Some(existing) = inventory_item_kinds::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
    else {
        return Err(loco_rs::Error::NotFound);
    };

    let AddItemKindPostParams {
        name,
        test_standard,
        default_checklist_id,
        default_interval_id,
        default_expiry_id,
        metadata_field_names,
    } = params;
    let metadata_field_names = inventory_item_kind_metadata_fields::normalize_metadata_field_names(
        metadata_field_names.unwrap_or_default(),
    );
    let item_ids = inventory_items::Entity::find()
        .filter(InventoryItemsColumn::InventoryItemKindId.eq(id))
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    let existing_metadata_values_by_item_id =
        load_metadata_value_maps_for_items(&ctx.db, &item_ids).await?;
    let trx = ctx.db.begin().await?;
    let mut item_kind: inventory_item_kinds::ActiveModel = existing.into();
    item_kind.name = ActiveValue::set(name.trim().to_string());
    item_kind.test_standard = ActiveValue::set(test_standard.trim().to_string());
    item_kind.default_checklist_id = ActiveValue::set(default_checklist_id);
    item_kind.default_interval_id = ActiveValue::set(default_interval_id);
    item_kind.default_expiry_id = ActiveValue::set(default_expiry_id);
    item_kind.update(&trx).await?;
    replace_metadata_fields_for_kind(&trx, id, &metadata_field_names).await?;
    rebuild_metadata_values_for_kind_items(
        &trx,
        id,
        &item_ids,
        existing_metadata_values_by_item_id,
    )
    .await?;
    trx.commit().await?;

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
        .add("/item/{id}/report.pdf", get(download_item_report))
        .add("/item/{id}/edit", get(edit_item))
        .add("/item/{id}/check", get(show_item_check))
        .add("/item/{id}/check", post(submit_item_check))
        .add("/item/{id}/edit", post(edit_item_post))
        .add("/item/{id}", delete(remove_item))
        .add("/item_kinds", get(list_item_kinds))
        .add("/item_kinds/{id}", get(show_item_kind))
        .add("/item_kinds/new", get(add_item_kind_new))
        .add("/item_kinds/new", post(add_item_kind_new_post))
        .add("/item_kinds/{id}/edit", get(edit_item_kind))
        .add("/item_kinds/{id}/edit", post(edit_item_kind_post))
        .add("/item_kinds/{id}", delete(remove_item_kind))
        .add("/list", get(list))
}

#[cfg(test)]
mod tests {
    use super::{
        build_inventory_metadata_columns, build_inventory_metadata_values_by_column_key,
        AddItemKindPostParams,
    };
    use crate::models::{inventory_item_kind_metadata_fields, inventory_items};
    use chrono::Utc;
    use std::collections::HashMap;

    fn test_inventory_item(id: i32, inventory_item_kind_id: i32) -> inventory_items::Model {
        inventory_items::Model {
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
            id,
            name: format!("Item {id}"),
            serial_number: None,
            last_checked_at: None,
            expiry: None,
            inventory_item_kind_id,
            checklist_id: 1,
            interval_id: 1,
        }
    }

    fn test_metadata_field(
        id: i32,
        inventory_item_kind_id: i32,
        position: i32,
        name: &str,
    ) -> inventory_item_kind_metadata_fields::Model {
        inventory_item_kind_metadata_fields::Model {
            created_at: Utc::now().into(),
            updated_at: Utc::now().into(),
            id,
            name: name.to_string(),
            position,
            inventory_item_kind_id,
        }
    }

    #[test]
    fn deserializes_single_metadata_field_name() {
        let params = serde_urlencoded::from_str::<AddItemKindPostParams>(
            "name=Device&test_standard=EN358&default_checklist_id=1&default_interval_id=1&default_expiry_id=1&metadata_field_names=manufacturer",
        )
        .unwrap();

        assert_eq!(
            params.metadata_field_names,
            Some(vec!["manufacturer".to_string()])
        );
    }

    #[test]
    fn deserializes_repeated_metadata_field_names() {
        let params = serde_urlencoded::from_str::<AddItemKindPostParams>(
            "name=Device&test_standard=EN358&default_checklist_id=1&default_interval_id=1&default_expiry_id=1&metadata_field_names=manufacturer&metadata_field_names=serialnr",
        )
        .unwrap();

        assert_eq!(
            params.metadata_field_names,
            Some(vec!["manufacturer".to_string(), "serialnr".to_string()])
        );
    }

    #[test]
    fn deduplicates_inventory_metadata_columns_across_item_kinds() {
        let items = vec![test_inventory_item(1, 10), test_inventory_item(2, 20)];
        let metadata_fields_by_kind_id = HashMap::from([
            (
                10,
                vec![
                    test_metadata_field(1, 10, 0, "Manufacturer"),
                    test_metadata_field(2, 10, 1, "Serialnr"),
                ],
            ),
            (
                20,
                vec![
                    test_metadata_field(3, 20, 0, "manufacturer"),
                    test_metadata_field(4, 20, 1, "Asset Tag"),
                ],
            ),
        ]);

        let columns = build_inventory_metadata_columns(&items, &metadata_fields_by_kind_id);

        assert_eq!(
            columns
                .into_iter()
                .map(|column| column.name)
                .collect::<Vec<_>>(),
            vec![
                "Manufacturer".to_string(),
                "Serialnr".to_string(),
                "Asset Tag".to_string(),
            ]
        );
    }

    #[test]
    fn combines_duplicate_metadata_field_values_into_one_cell() {
        let metadata_fields = vec![
            test_metadata_field(1, 10, 0, "Manufacturer"),
            test_metadata_field(2, 10, 1, "manufacturer"),
            test_metadata_field(3, 10, 2, "Serialnr"),
        ];
        let metadata_values_by_field_id = HashMap::from([
            (1, "Acme".to_string()),
            (2, "Beta".to_string()),
            (3, "SN-1".to_string()),
        ]);

        let values_by_column_key = build_inventory_metadata_values_by_column_key(
            &metadata_fields,
            &metadata_values_by_field_id,
        );

        assert_eq!(
            values_by_column_key.get("manufacturer"),
            Some(&"Acme, Beta".to_string())
        );
        assert_eq!(
            values_by_column_key.get("serialnr"),
            Some(&"SN-1".to_string())
        );
    }
}
