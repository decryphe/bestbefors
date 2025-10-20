#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use crate::models::translations::Entity;

#[debug_handler]
pub async fn list(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let translations = Entity::find().all(&ctx.db).await?;
    format::render().view(
        &v,
        "translations/list.html",
        data!({ "translations": translations }),
    )
}

pub fn routes() -> Routes {
    Routes::new().prefix("translations/").add("list", get(list))
}
