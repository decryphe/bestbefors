use std::{path::PathBuf, sync::LazyLock};

use chrono::{DateTime, FixedOffset, Utc};
use loco_rs::{Error, Result};
use typst::{
    foundations::{Array, Dict, IntoValue},
    layout::PagedDocument,
};
use typst_as_lib::{typst_kit_options::TypstKitFontOptions, TypstEngine};

const REPORT_TEMPLATE_PATH: &str = "assets/reports/single_item_history_main.typ";

static REPORT_ENGINE: LazyLock<TypstEngine> = LazyLock::new(|| {
    TypstEngine::builder()
        .with_file_system_resolver(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .search_fonts_with(
            TypstKitFontOptions::default()
                .include_system_fonts(true)
                .include_embedded_fonts(true),
        )
        .build()
});

#[derive(Clone, Debug)]
pub struct ReportField {
    pub label: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReportItem {
    pub name: String,
    pub serial_number: Option<String>,
    pub item_kind: Option<String>,
    pub checklist: Option<String>,
    pub interval: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_checked_at: Option<String>,
    pub expiry: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReportStep {
    pub position: i32,
    pub name: String,
    pub result: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReportCheck {
    pub checked_at: String,
    pub checked_by: Option<String>,
    pub overall_result: Option<String>,
    pub notes: Option<String>,
    pub steps: Vec<ReportStep>,
}

#[derive(Clone, Debug)]
pub struct SingleItemHistoryReport {
    pub title: String,
    pub generated_at: String,
    pub report_id: String,
    pub item: ReportItem,
    pub metadata: Vec<ReportField>,
    pub checks: Vec<ReportCheck>,
}

pub fn render_pdf(report: &SingleItemHistoryReport) -> Result<Vec<u8>> {
    let warned = REPORT_ENGINE
        .compile_with_input::<_, _, PagedDocument>(REPORT_TEMPLATE_PATH, build_input(report));

    for warning in warned.warnings {
        tracing::warn!(?warning, "typst report warning");
    }

    let document = warned
        .output
        .map_err(|error| Error::string(&format!("{error:?}")))?;

    typst_pdf::pdf(&document, &Default::default())
        .map_err(|error| Error::string(&format!("{error:?}")))
}

pub fn format_timestamp(value: DateTime<FixedOffset>) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}

pub fn format_generated_at(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d %H:%M UTC").to_string()
}

pub fn humanize_code(code: &str) -> String {
    let trimmed = code
        .strip_prefix("RESULT_")
        .or_else(|| code.strip_prefix("INTERVAL_"))
        .unwrap_or(code);

    trimmed
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let rest = chars.as_str().to_ascii_lowercase();
            format!("{}{}", first.to_ascii_uppercase(), rest)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_input(report: &SingleItemHistoryReport) -> Dict {
    let mut dict = Dict::new();
    insert_value(&mut dict, "title", report.title.clone());
    insert_value(&mut dict, "generated_at", report.generated_at.clone());
    insert_value(&mut dict, "report_id", report.report_id.clone());
    insert_value(&mut dict, "item", item_dict(&report.item));
    insert_value(
        &mut dict,
        "metadata",
        Array::from_iter(
            report
                .metadata
                .iter()
                .map(|field| field_dict(field).into_value()),
        ),
    );
    insert_value(
        &mut dict,
        "checks",
        Array::from_iter(
            report
                .checks
                .iter()
                .map(|check| check_dict(check).into_value()),
        ),
    );
    dict
}

fn field_dict(field: &ReportField) -> Dict {
    let mut dict = Dict::new();
    insert_value(&mut dict, "label", field.label.clone());
    insert_value(&mut dict, "value", field.value.clone());
    dict
}

fn item_dict(item: &ReportItem) -> Dict {
    let mut dict = Dict::new();
    insert_value(&mut dict, "name", item.name.clone());
    insert_value(&mut dict, "serial_number", item.serial_number.clone());
    insert_value(&mut dict, "item_kind", item.item_kind.clone());
    insert_value(&mut dict, "checklist", item.checklist.clone());
    insert_value(&mut dict, "interval", item.interval.clone());
    insert_value(&mut dict, "created_at", item.created_at.clone());
    insert_value(&mut dict, "updated_at", item.updated_at.clone());
    insert_value(&mut dict, "last_checked_at", item.last_checked_at.clone());
    insert_value(&mut dict, "expiry", item.expiry.clone());
    dict
}

fn step_dict(step: &ReportStep) -> Dict {
    let mut dict = Dict::new();
    insert_value(&mut dict, "position", step.position);
    insert_value(&mut dict, "name", step.name.clone());
    insert_value(&mut dict, "result", step.result.clone());
    insert_value(&mut dict, "notes", step.notes.clone());
    dict
}

fn check_dict(check: &ReportCheck) -> Dict {
    let mut dict = Dict::new();
    insert_value(&mut dict, "checked_at", check.checked_at.clone());
    insert_value(&mut dict, "checked_by", check.checked_by.clone());
    insert_value(&mut dict, "overall_result", check.overall_result.clone());
    insert_value(&mut dict, "notes", check.notes.clone());
    insert_value(
        &mut dict,
        "steps",
        Array::from_iter(check.steps.iter().map(|step| step_dict(step).into_value())),
    );
    dict
}

fn insert_value<T: IntoValue>(dict: &mut Dict, key: &str, value: T) {
    dict.insert(key.into(), value.into_value());
}
