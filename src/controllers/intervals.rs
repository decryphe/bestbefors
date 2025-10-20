#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use crate::models::intervals::Entity;

#[debug_handler]
pub async fn list(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let intervals = Entity::find().all(&ctx.db).await?;
    format::render().view(&v, "intervals/list.html", data!({ "intervals": intervals }))
}

pub fn routes() -> Routes {
    Routes::new().prefix("intervals/").add("list", get(list))
}
