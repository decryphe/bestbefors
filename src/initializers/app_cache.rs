use crate::models::{checklists, expiries, intervals, inventory_item_kinds, results, users};
use std::collections::BTreeMap;

pub struct AppCacheInitializer;

impl AppCacheInitializer {
    async fn reload_cached(ctx: &loco_rs::prelude::AppContext) -> loco_rs::Result<()> {
        use sea_orm::EntityTrait;

        let _: Option<BTreeMap<i32, checklists::Model>> = ctx.shared_store.remove();
        let checklists = checklists::Entity::find().all(&ctx.db).await?;
        let checklists: BTreeMap<_, _> = checklists.into_iter().map(|i| (i.id, i)).collect();
        ctx.shared_store.insert(checklists);

        let _: Option<BTreeMap<i32, intervals::Model>> = ctx.shared_store.remove();
        let intervals = intervals::Entity::find().all(&ctx.db).await?;
        let intervals: BTreeMap<_, _> = intervals.into_iter().map(|i| (i.id, i)).collect();
        ctx.shared_store.insert(intervals);

        let _: Option<BTreeMap<i32, expiries::Model>> = ctx.shared_store.remove();
        let expiries = expiries::Entity::find().all(&ctx.db).await?;
        let expiries: BTreeMap<_, _> = expiries.into_iter().map(|i| (i.id, i)).collect();
        ctx.shared_store.insert(expiries);

        refresh_item_kinds_cache(ctx).await?;

        let _: Option<BTreeMap<i32, results::Model>> = ctx.shared_store.remove();
        let results = results::Entity::find().all(&ctx.db).await?;
        let results: BTreeMap<_, _> = results.into_iter().map(|i| (i.id, i)).collect();
        ctx.shared_store.insert(results);

        refresh_users_cache(ctx).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl loco_rs::app::Initializer for AppCacheInitializer {
    fn name(&self) -> String {
        "app-cache".to_owned()
    }

    async fn before_run(&self, ctx: &loco_rs::prelude::AppContext) -> loco_rs::Result<()> {
        AppCacheInitializer::reload_cached(ctx).await
    }
}

pub trait AppData {
    fn get_checklists(&self) -> loco_rs::Result<BTreeMap<i32, checklists::Model>>;
    fn get_intervals(&self) -> loco_rs::Result<BTreeMap<i32, intervals::Model>>;
    fn get_expiries(&self) -> loco_rs::Result<BTreeMap<i32, expiries::Model>>;
    fn get_item_kinds(&self) -> loco_rs::Result<BTreeMap<i32, inventory_item_kinds::Model>>;
    fn get_results(&self) -> loco_rs::Result<BTreeMap<i32, results::Model>>;
    fn get_users(&self) -> loco_rs::Result<BTreeMap<i32, users::Model>>;
}

impl AppData for loco_rs::app::AppContext {
    fn get_checklists(&self) -> loco_rs::Result<BTreeMap<i32, checklists::Model>> {
        self.shared_store
            .get::<BTreeMap<i32, checklists::Model>>()
            .ok_or(loco_rs::Error::InternalServerError)
    }

    fn get_intervals(&self) -> loco_rs::Result<BTreeMap<i32, intervals::Model>> {
        self.shared_store
            .get::<BTreeMap<i32, intervals::Model>>()
            .ok_or(loco_rs::Error::InternalServerError)
    }

    fn get_expiries(&self) -> loco_rs::Result<BTreeMap<i32, expiries::Model>> {
        self.shared_store
            .get::<BTreeMap<i32, expiries::Model>>()
            .ok_or(loco_rs::Error::InternalServerError)
    }

    fn get_item_kinds(&self) -> loco_rs::Result<BTreeMap<i32, inventory_item_kinds::Model>> {
        self.shared_store
            .get::<BTreeMap<i32, inventory_item_kinds::Model>>()
            .ok_or(loco_rs::Error::InternalServerError)
    }

    fn get_results(&self) -> loco_rs::Result<BTreeMap<i32, results::Model>> {
        self.shared_store
            .get::<BTreeMap<i32, results::Model>>()
            .ok_or(loco_rs::Error::InternalServerError)
    }

    fn get_users(&self) -> loco_rs::Result<BTreeMap<i32, users::Model>> {
        self.shared_store
            .get::<BTreeMap<i32, users::Model>>()
            .ok_or(loco_rs::Error::InternalServerError)
    }
}

pub async fn refresh_item_kinds_cache(ctx: &loco_rs::prelude::AppContext) -> loco_rs::Result<()> {
    use sea_orm::EntityTrait;
    let item_kinds = inventory_item_kinds::Entity::find().all(&ctx.db).await?;
    let map: BTreeMap<_, _> = item_kinds.into_iter().map(|i| (i.id, i)).collect();
    let _: Option<BTreeMap<i32, inventory_item_kinds::Model>> = ctx.shared_store.remove();
    ctx.shared_store.insert(map);
    Ok(())
}

pub async fn refresh_users_cache(ctx: &loco_rs::prelude::AppContext) -> loco_rs::Result<()> {
    use sea_orm::EntityTrait;
    let users = users::Entity::find().all(&ctx.db).await?;
    let map: BTreeMap<_, _> = users.into_iter().map(|i| (i.id, i)).collect();
    let _: Option<BTreeMap<i32, users::Model>> = ctx.shared_store.remove();
    ctx.shared_store.insert(map);
    Ok(())
}
