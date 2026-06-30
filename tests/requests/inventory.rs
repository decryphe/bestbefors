use axum::http::header;
use axum_test::TestServer;
use bestbefors::{
    app::App,
    models::{
        _entities::checklist_steps::Column as ChecklistStepsColumn, checklist_steps,
        executed_checklist_steps, executed_checklists, inventory_item_check_steps,
        inventory_item_checks, inventory_item_kind_metadata_fields, inventory_item_kinds,
        inventory_item_metadata_values, inventory_items,
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn downloads_single_item_history_pdf() {
    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    let now = chrono::Utc::now();
    let kind = inventory_item_kinds::ActiveModel {
        id: ActiveValue::not_set(),
        name: ActiveValue::set("Harness".to_string()),
        test_standard: ActiveValue::set("EN 358".to_string()),
        default_checklist_id: ActiveValue::set(2),
        default_interval_id: ActiveValue::set(4),
        default_expiry_id: ActiveValue::set(1),
        created_at: ActiveValue::set(now.into()),
        updated_at: ActiveValue::set(now.into()),
    }
    .insert(&boot.app_context.db)
    .await
    .unwrap();

    let metadata_field = inventory_item_kind_metadata_fields::ActiveModel {
        id: ActiveValue::not_set(),
        name: ActiveValue::set("Manufacturer".to_string()),
        position: ActiveValue::set(1),
        inventory_item_kind_id: ActiveValue::set(kind.id),
        created_at: ActiveValue::set(now.into()),
        updated_at: ActiveValue::set(now.into()),
    }
    .insert(&boot.app_context.db)
    .await
    .unwrap();

    let item = inventory_items::ActiveModel {
        id: ActiveValue::not_set(),
        name: ActiveValue::set("Harness A".to_string()),
        serial_number: ActiveValue::set(Some("SN-42".to_string())),
        last_checked_at: ActiveValue::set(Some(now.into())),
        expiry: ActiveValue::set(Some(now.into())),
        inventory_item_kind_id: ActiveValue::set(kind.id),
        checklist_id: ActiveValue::set(2),
        interval_id: ActiveValue::set(4),
        created_at: ActiveValue::set(now.into()),
        updated_at: ActiveValue::set(now.into()),
    }
    .insert(&boot.app_context.db)
    .await
    .unwrap();

    inventory_item_metadata_values::ActiveModel {
        id: ActiveValue::not_set(),
        value: ActiveValue::set("AustriAlpin".to_string()),
        inventory_item_id: ActiveValue::set(item.id),
        inventory_item_kind_metadata_field_id: ActiveValue::set(metadata_field.id),
        created_at: ActiveValue::set(now.into()),
        updated_at: ActiveValue::set(now.into()),
    }
    .insert(&boot.app_context.db)
    .await
    .unwrap();

    let executed_checklist = executed_checklists::ActiveModel {
        id: ActiveValue::not_set(),
        name: ActiveValue::set("Annual PPE Inspection - Harness A".to_string()),
        description: ActiveValue::set(Some("Generated in test".to_string())),
        created_at: ActiveValue::set(now.into()),
        updated_at: ActiveValue::set(now.into()),
    }
    .insert(&boot.app_context.db)
    .await
    .unwrap();

    let template_steps = checklist_steps::Entity::find()
        .filter(ChecklistStepsColumn::ChecklistId.eq(2))
        .all(&boot.app_context.db)
        .await
        .unwrap();
    assert!(
        !template_steps.is_empty(),
        "expected seeded checklist steps"
    );

    let mut executed_steps = Vec::new();
    for step in template_steps {
        let executed_step = executed_checklist_steps::ActiveModel {
            id: ActiveValue::not_set(),
            position: ActiveValue::set(step.position),
            name: ActiveValue::set(step.name),
            description: ActiveValue::set(step.description),
            executed_checklist_id: ActiveValue::set(executed_checklist.id),
            created_at: ActiveValue::set(now.into()),
            updated_at: ActiveValue::set(now.into()),
        }
        .insert(&boot.app_context.db)
        .await
        .unwrap();
        executed_steps.push(executed_step);
    }

    let check = inventory_item_checks::ActiveModel {
        id: ActiveValue::not_set(),
        finished: ActiveValue::set(true),
        checked_at: ActiveValue::set(now.into()),
        notes: ActiveValue::set(Some("Overall notes".to_string())),
        inventory_item_id: ActiveValue::set(item.id),
        executed_checklist_id: ActiveValue::set(executed_checklist.id),
        checked_by: ActiveValue::set(1),
        result_id: ActiveValue::set(1),
        created_at: ActiveValue::set(now.into()),
        updated_at: ActiveValue::set(now.into()),
    }
    .insert(&boot.app_context.db)
    .await
    .unwrap();

    for executed_step in executed_steps {
        inventory_item_check_steps::ActiveModel {
            id: ActiveValue::not_set(),
            notes: ActiveValue::set(Some("Step notes".to_string())),
            inventory_item_check_id: ActiveValue::set(check.id),
            executed_checklist_step_id: ActiveValue::set(executed_step.id),
            result_id: ActiveValue::set(1),
            created_at: ActiveValue::set(now.into()),
            updated_at: ActiveValue::set(now.into()),
        }
        .insert(&boot.app_context.db)
        .await
        .unwrap();
    }

    let server = TestServer::builder()
        .mock_transport()
        .build(boot.router.clone().unwrap())
        .unwrap();
    let response = server
        .get(&format!("/inventory/item/{}/report.pdf", item.id))
        .await;

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/pdf")
    );
    assert!(
        response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("inventory-item-")),
        "expected content-disposition attachment header"
    );
    assert!(
        response.as_bytes().starts_with(b"%PDF-"),
        "expected response body to start with PDF signature"
    );
}
