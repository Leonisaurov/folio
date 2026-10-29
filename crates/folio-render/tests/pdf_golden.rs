//! Golden de texto extraído del PDF y comprobación de páginas.

use std::path::PathBuf;
use std::process::Command;

use folio_core::model::{DrawingContent, Paragraph, TableContent};
use folio_core::{Block, BlockContent, FixedClock, Note};
use folio_render::pdf::{PdfOptions, write_pdf, write_pdf_with_assets};

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/sample.pdf.txt")
}

fn sample() -> Note {
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut note = Note::new("Informe", &clock);
    note.blocks.push(Block::text(vec![
        Paragraph::styled("h1", "Informe de prueba"),
        Paragraph::body("Texto normal con una línea larga para probar el paginado del documento."),
        Paragraph::styled("quote", "Una cita"),
        Paragraph::styled("list_item", "Elemento de lista"),
    ]));
    let mut table = TableContent::new(2, 2);
    table.set_cell(0, 0, "A");
    table.set_cell(0, 1, "B");
    table.set_cell(1, 0, "1");
    table.set_cell(1, 1, "2");
    note.blocks
        .push(Block::new("table", BlockContent::Table(table)));
    note.blocks.push(Block::new(
        "drawing",
        BlockContent::Drawing(DrawingContent {
            strokes: vec![folio_core::model::Stroke {
                points: vec![vec![10.0, 10.0], vec![100.0, 80.0]],
                ..Default::default()
            }],
            ..Default::default()
        }),
    ));
    note
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn pdf_golden_text_and_pages() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("informe.pdf");
    write_pdf(&pdf, &sample(), &PdfOptions::default()).unwrap();
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-1."));
    assert!(bytes.len() > 1000);

    if let Ok(info) = Command::new("pdfinfo").arg(&pdf).output() {
        let text = String::from_utf8_lossy(&info.stdout);
        assert!(text.contains("Pages:"));
        assert!(!text.contains("Pages:          0"));
    }

    let Ok(output) = Command::new("pdftotext")
        .args(["-layout", pdf.to_str().unwrap(), "-"])
        .output()
    else {
        eprintln!("pdftotext no disponible; se omite el golden de texto");
        return;
    };
    assert!(output.status.success());
    let text = normalize(&String::from_utf8_lossy(&output.stdout));
    assert!(text.contains("Informe de prueba"));
    assert!(text.contains("Texto normal"));
    assert!(text.contains("Una cita"));
    assert!(text.contains("Elemento de lista"));

    let path = golden_path();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("{text}\n")).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(format!("{text}\n"), expected, "golden PDF distinto");
}

#[test]
fn free_blocks_create_a_canvas_page() {
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut note = Note::new("Lienzo", &clock);
    for index in 0..12 {
        let mut block = Block::text_plain(&format!("bloque {index}"));
        block.placement = folio_core::Placement::free(
            100.0 + index as f64 * 120.0,
            80.0 + (index % 3) as f64 * 100.0,
            100.0,
            60.0,
        );
        note.blocks.push(block);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("canvas.pdf");
    write_pdf(&path, &note, &PdfOptions::default()).unwrap();
    let output = Command::new("pdftotext")
        .args(["-layout", path.to_str().unwrap(), "-"])
        .output();
    if let Ok(output) = output {
        let text = String::from_utf8_lossy(&output.stdout);
        let compact: String = text.split_whitespace().collect();
        assert!(compact.contains("Lienzo"));
        assert!(compact.contains("text@"));
        // El ancho de las tarjetas parte alguna palabra en la extracción; lo
        // importante es que el bloque 11 está representado.
        assert!(compact.contains("e11"));
    }
}

#[test]
fn long_note_paginates_and_media_resolves() {
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut note = Note::new("Largo", &clock);
    for index in 0..160 {
        note.blocks.push(Block::text_plain(&format!(
            "Párrafo {index} con texto suficiente para forzar varias páginas en el PDF."
        )));
    }
    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("pixel.png");
    let image = image::RgbaImage::from_pixel(16, 16, image::Rgba([32, 128, 220, 255]));
    image.save(&image_path).unwrap();
    note.blocks.push(Block::new(
        "image",
        BlockContent::Image(folio_core::model::ImageContent {
            asset: "a".repeat(64),
            caption: "Imagen de prueba".into(),
            ..Default::default()
        }),
    ));
    let pdf = dir.path().join("largo.pdf");
    let resolver = |_hash: &str| Some(image_path.clone());
    write_pdf_with_assets(&pdf, &note, &PdfOptions::default(), &resolver).unwrap();
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-1."));
    if let Ok(info) = Command::new("pdfinfo").arg(&pdf).output() {
        let text = String::from_utf8_lossy(&info.stdout);
        let pages: u32 = text
            .lines()
            .find_map(|line| line.strip_prefix("Pages:"))
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0);
        assert!(pages >= 2, "se esperaban varias páginas, hay {pages}");
    }
}

#[test]
fn encrypted_style_asset_callback_is_used() {
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut note = Note::new("Asset", &clock);
    note.blocks.push(Block::new(
        "image",
        BlockContent::Image(folio_core::model::ImageContent {
            asset: "b".repeat(64),
            caption: "Desde callback".into(),
            ..Default::default()
        }),
    ));
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("a.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([200, 20, 20, 255]))
        .save(&png)
        .unwrap();
    let pdf = dir.path().join("asset.pdf");
    let resolver = |_hash: &str| Some(png.clone());
    write_pdf_with_assets(&pdf, &note, &PdfOptions::default(), &resolver).unwrap();
    assert!(std::fs::metadata(pdf).unwrap().len() > 500);
}
