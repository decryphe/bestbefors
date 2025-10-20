#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use crate::models::expiries::Entity;

#[debug_handler]
pub async fn list(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let expiries = Entity::find().all(&ctx.db).await?;
    format::render().view(&v, "expiries/list.html", data!({ "expiries": expiries }))
}

pub fn routes() -> Routes {
    Routes::new().prefix("expiries/").add("list", get(list))
}
