use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

#[allow(unused_imports)]
use crate::{
    controllers, initializers,
    models::_entities::{
        checklist_steps, checklists, expiries, intervals, results, translations, users,
    },
    tasks,
    workers::downloader::DownloadWorker,
};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![
            Box::new(initializers::view_engine::ViewEngineInitializer),
            Box::new(initializers::app_cache::AppCacheInitializer),
        ])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
            .add_route(controllers::checklists::routes())
            .add_route(controllers::expiries::routes())
            .add_route(controllers::home::routes())
            .add_route(controllers::intervals::routes())
            .add_route(controllers::inventory::routes())
            .add_route(controllers::users::routes())
            .add_route(controllers::translations::routes())
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        db::seed::<expiries::ActiveModel>(
            &ctx.db,
            &base.join("expiries.yaml").display().to_string(),
        )
        .await?;
        db::seed::<intervals::ActiveModel>(
            &ctx.db,
            &base.join("intervals.yaml").display().to_string(),
        )
        .await?;
        db::seed::<results::ActiveModel>(&ctx.db, &base.join("results.yaml").display().to_string())
            .await?;
        db::seed::<translations::ActiveModel>(
            &ctx.db,
            &base.join("translations.yaml").display().to_string(),
        )
        .await?;

        db::seed::<checklists::ActiveModel>(
            &ctx.db,
            &base.join("checklists.yaml").display().to_string(),
        )
        .await?;
        db::seed::<checklist_steps::ActiveModel>(
            &ctx.db,
            &base.join("checklist_steps.yaml").display().to_string(),
        )
        .await?;
        Ok(())
    }
}
