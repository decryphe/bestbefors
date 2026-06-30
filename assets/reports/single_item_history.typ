// Reusable Typst template for a single inventory item's check history report.
//
// Expected data shape:
// #let report = (
//   title: "Inventory Item Check History",
//   generated_at: "2025-08-16 09:45",
//   report_id: "REP-2025-0001",
//   item: (
//     name: "Harness A",
//     serial_number: "SN-12345",
//     item_kind: "Safety Harness",
//     checklist: "Annual PPE Inspection",
//     interval: "INTERVAL_12_MONTHS",
//     created_at: "2024-04-04 08:00",
//     updated_at: "2025-08-16 09:45",
//     last_checked_at: "2025-08-16 08:30",
//     expiry: "2028-03-31",
//   ),
//   metadata: (
//     (label: "Manufacturer", value: "AustriAlpin"),
//     (label: "Location", value: "Station 1"),
//   ),
//   checks: (
//     (
//       checked_at: "2025-08-16 08:30",
//       checked_by: "Inspector Name",
//       overall_result: "PASS",
//       notes: "General condition acceptable.",
//       steps: (
//         (
//           position: 1,
//           name: "Labels present",
//           description: "Verify all labels are readable.",
//           result: "PASS",
//           notes: "",
//         ),
//       ),
//     ),
//   ),
// )

#set page(paper: "a4", margin: (x: 10mm, y: 10mm))
#set text(font: "Liberation Sans", size: 9pt)
#set par(justify: false)

#let placeholder = "—"

#let display(value) = if value == none or value == "" {
  placeholder
} else {
  value
}

#let render_details_table(fields) = table(
  columns: (48mm, 1fr),
  stroke: 0.5pt,
  inset: 4pt,
  ..fields
    .map(field => (
      [*#display(field.label)*],
      [#display(field.value)],
    ))
    .flatten(),
)

#let step_header(step) = if step.position == none {
  display(step.name)
} else {
  str(step.position) + ". " + display(step.name)
}

#let combined_notes(check) = {
  let check_notes = if check.notes == none or check.notes == "" {
    ()
  } else {
    (check.notes,)
  }
  let step_notes = check
    .steps
    .filter(step => step.notes != none and step.notes != "")
    .map(step => str(step.position) + ". " + step.notes)
  let notes = (..check_notes, ..step_notes)

  if notes.len() == 0 {
    placeholder
  } else {
    notes.join(" | ")
  }
}

#let render_check_history_table(checks) = {
  let step_columns = if checks.len() == 0 { () } else { checks.at(0).steps }

  table(
    columns: step_columns.len() + 4,
    stroke: 0.5pt,
    inset: 3pt,
    table.header(
      [*Checked At*],
      [*Checked By*],
      [*Overall*],
      ..step_columns.map(step => [*#step_header(step)*]),
      [*Notes*],
    ),
    ..checks
      .map(check => (
        [#display(check.checked_at)],
        [#display(check.checked_by)],
        [#display(check.overall_result)],
        ..step_columns
          .enumerate()
          .map(((index, _step)) => {
            let value = if index < check.steps.len() {
              check.steps.at(index).result
            } else {
              none
            }

            [#display(value)]
          }),
        [#combined_notes(check)],
      ))
      .flatten(),
  )
}

#let render_single_item_history_report(report) = [
  #align(center)[#text(size: 16pt, weight: "bold")[#display(report.title)]]
  #v(4pt)

  #align(center)[
    Generated at: #display(report.generated_at) \
    Report ID: #display(report.report_id)
  ]

  #v(10pt)
  #text(weight: "bold", size: 12pt)[Item Details]
  #v(4pt)

  #render_details_table((
    (label: "Inventory Item", value: report.item.name),
    (label: "Serial Number", value: report.item.serial_number),
    (label: "Item Kind", value: report.item.item_kind),
    (label: "Checklist", value: report.item.checklist),
    (label: "Interval", value: report.item.interval),
    (label: "Last Checked", value: report.item.last_checked_at),
    (label: "Created At", value: report.item.created_at),
    (label: "Updated At", value: report.item.updated_at),
    (label: "Expiry", value: report.item.expiry),
    ..report.metadata.map(field => (label: field.label, value: field.value)),
  ))

  #v(10pt)
  #text(weight: "bold", size: 12pt)[Check History]
  #v(4pt)

  #if report.checks.len() == 0 [
    No checks have been recorded for this inventory item.
  ] else [
    #render_check_history_table(report.checks)
  ]
]
