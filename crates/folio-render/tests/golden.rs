//! Golden files de exportación. Actualizar con `UPDATE_GOLDEN=1 cargo test -p folio-render --test golden`.

use std::fs;
use std::path::PathBuf;

use folio_core::model::{ImageContent, Paragraph, Span, TableContent, TextContent, WebContent};
use folio_core::{Block, BlockContent, FixedClock, Note};
use folio_render::{ExportOptions, to_html, to_markdown};

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

fn sample() -> Note {
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut note = Note::new("Jardín", &clock);
    note.meta.tags = vec!["casa".into()];
    let mut paragraph = Paragraph::body("Hola ");
    paragraph.spans.push(Span::marked("mundo", &["bold"]));
    paragraph.spans.push(Span::plain(" y "));
    paragraph.spans.push(Span {
        text: "enlace".into(),
        marks: vec![],
        link: Some("https://example.com".into()),
        color: None,
        extra: Default::default(),
    });
    note.blocks.push(Block::new(
        "text",
        BlockContent::Text(TextContent {
            paragraphs: vec![
                Paragraph::styled("h2", "Sección"),
                paragraph,
                Paragraph::styled("quote", "Una cita"),
                {
                    let mut item = Paragraph::styled("checklist_item", "hecho");
                    item.checked = Some(true);
                    item
                },
            ],
            ..Default::default()
        }),
    ));
    let mut table = TableContent::new(2, 2);
    table.set_cell(0, 0, "A");
    table.set_cell(0, 1, "B");
    table.set_cell(1, 0, "1");
    table.set_cell(1, 1, "2");
    note.blocks
        .push(Block::new("table", BlockContent::Table(table)));
    note.blocks.push(Block::new(
        "image",
        BlockContent::Image(ImageContent {
            asset: "a".repeat(64),
            caption: "Plano".into(),
            alt: "Croquis".into(),
            ..Default::default()
        }),
    ));
    note.blocks.push(Block::new(
        "web",
        BlockContent::Web(WebContent {
            url: "https://example.com/articulo".into(),
            title: "Artículo".into(),
            description: "Resumen".into(),
            ..Default::default()
        }),
    ));
    note
}

fn check(name: &str, actual: String) {
    let path = golden_path(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("falta {}; ejecuta UPDATE_GOLDEN=1", path.display()));
    assert_eq!(actual, expected, "golden distinto: {}", path.display());
}

#[test]
fn golden_markdown_and_html() {
    let note = sample();
    let options = ExportOptions::default();
    check("sample.md.golden", to_markdown(&note, &options));
    check("sample.html", to_html(&note, &options));
}

#[test]
fn golden_has_no_scripts_and_keeps_canvas() {
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut note = Note::new("Canvas", &clock);
    let mut block = Block::text_plain("libre");
    block.placement = folio_core::Placement::free(120.0, 80.0, 300.0, 100.0);
    note.blocks.push(block);
    let options = ExportOptions::default();
    let markdown = to_markdown(&note, &options);
    check("canvas.md.golden", markdown);
    let html = to_html(&note, &options);
    assert!(html.contains("position:absolute;left:120.0px;top:80.0px"));
    assert!(!html.contains("<script"));
}
