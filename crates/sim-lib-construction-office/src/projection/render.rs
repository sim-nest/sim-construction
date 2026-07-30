//! Rendering of filtered pack rows into existing office value types.

use sim_kernel::{Cx, Expr};
use sim_lib_deck::{Deck, Slide, SlideBlock};
use sim_lib_doc_core::{Doc, DocId, DocKind};
use sim_lib_sheet::{CellRef, CellValue, Sheet};

use super::{Metadata, StatusRow, evidence_text, status_label};
use crate::{OfficePackError, PackSection};

pub(super) fn report_doc(
    cx: &mut Cx,
    id: DocId,
    metadata: &Metadata,
    rows: &[StatusRow],
) -> Result<Doc, OfficePackError> {
    let body = Expr::Map(vec![
        field(
            "kind",
            Expr::Symbol(sim_kernel::Symbol::qualified("construction", "office-pack")),
        ),
        field("metadata", metadata_expr(metadata)),
        field("sections", sections_expr(rows)),
    ]);
    let body = cx.factory().expr(body)?;
    Ok(Doc::new(
        DocKind::new("report"),
        id,
        body,
        metadata.evidence.clone(),
    ))
}

fn metadata_expr(metadata: &Metadata) -> Expr {
    Expr::Map(
        metadata
            .fields()
            .into_iter()
            .map(|(key, value)| field(key, Expr::String(value)))
            .collect(),
    )
}

fn sections_expr(rows: &[StatusRow]) -> Expr {
    Expr::List(
        PackSection::ORDERED
            .into_iter()
            .filter_map(|section| {
                let section_rows = rows
                    .iter()
                    .filter(|row| row.section == section)
                    .map(row_expr)
                    .collect::<Vec<_>>();
                (!section_rows.is_empty()).then(|| {
                    Expr::Map(vec![
                        field("section", Expr::String(section.as_str().to_owned())),
                        field("rows", Expr::List(section_rows)),
                    ])
                })
            })
            .collect(),
    )
}

fn row_expr(row: &StatusRow) -> Expr {
    Expr::Map(vec![
        field("control-id", Expr::String(row.control.to_string())),
        field("value", Expr::String(row.value.clone())),
        field(
            "status-value",
            Expr::String(status_label(row.status).to_owned()),
        ),
        field("status-explanation", Expr::String(row.explanation.clone())),
        field("mandatory", Expr::Bool(row.mandatory)),
        field("changed-since-meeting", Expr::Bool(row.changed)),
        field("evidence-refs", Expr::String(evidence_text(&row.evidence))),
    ])
}

pub(super) fn status_sheet(metadata: &Metadata, rows: &[StatusRow]) -> Sheet {
    let mut sheet = Sheet::new(format!("{} {}", metadata.project, metadata.cadence));
    let mut row = 1_u32;
    for (name, value) in metadata.fields() {
        set_text(&mut sheet, 1, row, name);
        set_text(&mut sheet, 2, row, value);
        row += 1;
    }
    row += 1;
    for (column, heading) in [
        "Section",
        "Control",
        "Value",
        "Status",
        "Explanation",
        "Mandatory",
        "Changed",
        "Evidence refs",
    ]
    .into_iter()
    .enumerate()
    {
        set_text(&mut sheet, column as u32 + 1, row, heading);
    }
    for status in rows {
        row += 1;
        for (column, value) in [
            status.section.as_str().to_owned(),
            status.control.to_string(),
            status.value.clone(),
            status_label(status.status).to_owned(),
            status.explanation.clone(),
            status.mandatory.to_string(),
            status.changed.to_string(),
            evidence_text(&status.evidence),
        ]
        .into_iter()
        .enumerate()
        {
            set_text(&mut sheet, column as u32 + 1, row, value);
        }
    }
    sheet
}

pub(super) fn status_deck(metadata: &Metadata, rows: &[StatusRow]) -> Deck {
    let mut deck = Deck::new(format!(
        "{} {} {} pack",
        metadata.project, metadata.role, metadata.cadence
    ));
    let mut provenance = Slide::new("provenance", "Authority and provenance");
    provenance.push_block(SlideBlock::Table {
        columns: vec!["Field".to_owned(), "Value".to_owned()],
        rows: metadata
            .fields()
            .into_iter()
            .map(|(name, value)| vec![name.to_owned(), value])
            .collect(),
    });
    deck.push_slide(provenance);
    for section in PackSection::ORDERED {
        let section_rows = rows
            .iter()
            .filter(|row| row.section == section)
            .map(|row| {
                format!(
                    "{} | {} | {}: {} | changed={}",
                    row.control,
                    row.value,
                    status_label(row.status),
                    row.explanation,
                    row.changed
                )
            })
            .collect::<Vec<_>>();
        if !section_rows.is_empty() {
            let mut slide = Slide::new(section.as_str(), section.as_str());
            slide.push_block(SlideBlock::BulletList(section_rows));
            deck.push_slide(slide);
        }
    }
    deck
}

fn set_text(sheet: &mut Sheet, column: u32, row: u32, value: impl Into<String>) {
    sheet.set_cell(
        CellRef::new(column, row).expect("positive office pack cell"),
        CellValue::Text(value.into()),
    );
}

fn field(name: &str, value: Expr) -> (Expr, Expr) {
    (
        Expr::Symbol(sim_kernel::Symbol::qualified("construction-pack", name)),
        value,
    )
}
