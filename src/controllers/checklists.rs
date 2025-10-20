#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde::Deserialize;

use crate::models::{checklist_steps, checklists, inventory_items};

#[derive(serde::Serialize)]
struct ChecklistWithSteps {
    checklist: checklists::Model,
    steps: Vec<checklist_steps::Model>,
}

#[debug_handler]
pub async fn list(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let checklists = checklists::Entity::find()
        .find_with_related(checklist_steps::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|(checklist, mut steps)| {
            steps.sort_unstable_by_key(|step| step.position);
            ChecklistWithSteps { checklist, steps }
        })
        .collect::<Vec<_>>();
    format::render().view(
        &v,
        "checklists/list.html",
        data!({ "checklists": checklists }),
    )
}

#[debug_handler]
pub async fn add(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "checklists/add.html", data!({}))
}

#[derive(Debug, Deserialize)]
pub struct ChecklistStepInput {
    pub position: i32,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct AddChecklistParams {
    pub name: String,
    pub description: String,
    pub steps: Vec<ChecklistStepInput>,
}

#[debug_handler]
pub async fn add_post(
    State(ctx): State<AppContext>,
    Json(params): Json<AddChecklistParams>,
) -> Result<Response> {
    tracing::warn!("received post: {params:?}");
    let name = params.name.trim();
    if name.is_empty() {
        return Err(loco_rs::Error::BadRequest(
            "Checklist name must not be empty".to_string(),
        ));
    }
    let description = Some(params.description.trim())
        .filter(|s| s.is_empty())
        .map(str::to_string);

    let mut prepared_steps: Vec<_> = params
        .steps
        .into_iter()
        .map(|step| {
            let description = Some(step.description.trim())
                .filter(|s| s.is_empty())
                .map(str::to_string);
            (step.position, step.name.trim().to_owned(), description)
        })
        .collect();

    if prepared_steps.is_empty() {
        return Err(loco_rs::Error::BadRequest(
            "Please provide at least one checklist step".to_string(),
        ));
    }

    if !has_unique_elements(prepared_steps.iter().map(|(pos, _, _)| pos)) {
        return Err(loco_rs::Error::BadRequest(
            "Checklist steps must all have unique positions".to_string(),
        ));
    }

    prepared_steps.sort_by_key(|(position, _, _)| *position);

    let checklist = checklists::ActiveModel {
        name: ActiveValue::set(name.to_string()),
        description: ActiveValue::set(description),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;

    for (position, step_name, description) in prepared_steps {
        let step = checklist_steps::ActiveModel {
            checklist_id: ActiveValue::set(checklist.id),
            position: ActiveValue::set(position),
            name: ActiveValue::set(step_name),
            description: ActiveValue::set(description),
            ..Default::default()
        };
        step.insert(&ctx.db).await?;
    }

    format::redirect("/checklists/list")
}

#[debug_handler]
pub async fn remove(State(ctx): State<AppContext>, Path(id): Path<i32>) -> Result<Response> {
    use sea_orm::PaginatorTrait;
    let usage_count = inventory_items::Entity::find()
        .filter(crate::models::_entities::inventory_items::Column::ChecklistId.eq(id))
        .count(&ctx.db)
        .await?;

    if usage_count > 0 {
        return Err(loco_rs::Error::BadRequest(
            "Checklist is in use by inventory items".to_string(),
        ));
    }

    let deleted = checklists::Entity::delete_by_id(id).exec(&ctx.db).await?;
    if deleted.rows_affected == 0 {
        return Err(loco_rs::Error::NotFound);
    }

    format::json(data!({ "status": "ok" }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("checklists/")
        .add("add", get(add))
        .add("add", post(add_post))
        .add("{id}", delete(remove))
        .add("list", get(list))
}

fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + std::hash::Hash,
{
    let mut uniq = std::collections::HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}
