//! Exportación PDF con Cairo/Pango, paginación de flujo y página de lienzo.
//!
//! No depende de GTK ni de WebKit. Las imágenes se resuelven mediante un
//! callback que el llamador conecta con la bóveda (para descifrarlas si hace
//! falta). Nunca se renderiza una vista web viva: solo la tarjeta.

use std::path::{Path, PathBuf};

use cairo::{Context, Format, ImageSurface, PdfMetadata, PdfSurface};
use folio_core::model::{
    BlockContent, DrawingContent, ImageContent, Paragraph, PluginContent, TableContent, WebContent,
};
use folio_core::{Block, FolioError, Note};
use pango::{FontDescription, WrapMode};
use pangocairo::functions::{create_layout, show_layout};

pub const A4_WIDTH: f64 = 595.0;
pub const A4_HEIGHT: f64 = 842.0;
pub const MARGIN: f64 = 48.0;
const FONT_SCALE: i32 = pango::SCALE;

#[derive(Debug, Clone)]
pub struct PdfOptions {
    pub page_width: f64,
    pub page_height: f64,
    pub margin: f64,
    pub include_canvas: bool,
    pub title: Option<String>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            page_width: A4_WIDTH,
            page_height: A4_HEIGHT,
            margin: MARGIN,
            include_canvas: true,
            title: None,
        }
    }
}

/// Resuelve un hash de asset a un archivo descifrado temporal (o definitivo).
pub type AssetResolver<'a> = dyn Fn(&str) -> Option<PathBuf> + 'a;

fn pdf_error(error: impl std::fmt::Display) -> FolioError {
    FolioError::Invalid(format!("PDF: {error}"))
}

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn paragraph_markup(paragraph: &Paragraph) -> String {
    let mut out = String::new();
    for span in &paragraph.spans {
        let mut text = escape_markup(&span.text);
        for mark in &span.marks {
            text = match mark.as_str() {
                "bold" => format!("<b>{text}</b>"),
                "italic" => format!("<i>{text}</i>"),
                "underline" => format!("<u>{text}</u>"),
                "strikethrough" => format!("<s>{text}</s>"),
                "code" => format!("<tt>{text}</tt>"),
                "highlight" => format!("<span background=\"#f9e27d\">{text}</span>"),
                _ => text,
            };
        }
        if let Some(link) = &span.link {
            text = format!("{text} ({})", escape_markup(link));
        }
        out.push_str(&text);
    }
    if out.is_empty() { " ".into() } else { out }
}

fn font_for(style: &str) -> FontDescription {
    let mut font = FontDescription::from_string("Sans 11");
    match style {
        "h1" => font = FontDescription::from_string("Sans Bold 20"),
        "h2" => font = FontDescription::from_string("Sans Bold 15"),
        "h3" => font = FontDescription::from_string("Sans Bold 12"),
        "quote" => font = FontDescription::from_string("Sans Italic 11"),
        "code" => font = FontDescription::from_string("Monospace 9"),
        _ => {}
    }
    font
}

fn paragraph_height(cr: &Context, paragraph: &Paragraph, width: f64) -> f64 {
    let layout = create_layout(cr);
    layout.set_width((width * FONT_SCALE as f64) as i32);
    layout.set_wrap(WrapMode::WordChar);
    layout.set_font_description(Some(&font_for(&paragraph.style)));
    layout.set_markup(&paragraph_markup(paragraph));
    layout.pixel_size().1 as f64 + 4.0
}

fn draw_paragraph(cr: &Context, paragraph: &Paragraph, x: f64, y: f64, width: f64) -> f64 {
    let layout = create_layout(cr);
    layout.set_width((width * FONT_SCALE as f64) as i32);
    layout.set_wrap(WrapMode::WordChar);
    layout.set_font_description(Some(&font_for(&paragraph.style)));
    layout.set_markup(&paragraph_markup(paragraph));
    let height = layout.pixel_size().1 as f64 + 4.0;
    cr.move_to(x, y);
    show_layout(cr, &layout);
    height
}

struct PdfWriter<'a> {
    cr: Context,
    surface: PdfSurface,
    options: PdfOptions,
    page: u32,
    y: f64,
    resolver: &'a AssetResolver<'a>,
}

impl<'a> PdfWriter<'a> {
    fn content_width(&self) -> f64 {
        self.options.page_width - self.options.margin * 2.0
    }

    fn content_bottom(&self) -> f64 {
        self.options.page_height - self.options.margin - 16.0
    }

    fn ensure_space(&mut self, height: f64) {
        if self.y + height > self.content_bottom() && self.y > self.options.margin {
            self.show_page();
        }
    }

    fn show_page(&mut self) {
        self.footer();
        let _ = self.cr.show_page();
        self.page += 1;
        self.y = self.options.margin;
        self.page_header();
    }

    fn page_header(&mut self) {
        let title = self.options.title.clone().unwrap_or_else(|| "Folio".into());
        let layout = create_layout(&self.cr);
        layout.set_font_description(Some(&FontDescription::from_string("Sans 8")));
        layout.set_text(&title);
        self.cr.set_source_rgb(0.45, 0.45, 0.45);
        self.cr.move_to(self.options.margin, 18.0);
        show_layout(&self.cr, &layout);
        self.cr.set_source_rgb(0.1, 0.1, 0.1);
    }

    fn footer(&mut self) {
        let layout = create_layout(&self.cr);
        layout.set_font_description(Some(&FontDescription::from_string("Sans 8")));
        layout.set_text(&format!("Folio · página {}", self.page));
        self.cr.set_source_rgb(0.45, 0.45, 0.45);
        self.cr
            .move_to(self.options.margin, self.options.page_height - 24.0);
        show_layout(&self.cr, &layout);
        self.cr.set_source_rgb(0.1, 0.1, 0.1);
    }

    fn draw_text(&mut self, content: &folio_core::model::TextContent) {
        for paragraph in &content.paragraphs {
            let height = paragraph_height(&self.cr, paragraph, self.content_width());
            self.ensure_space(height);
            self.y += draw_paragraph(
                &self.cr,
                paragraph,
                self.options.margin,
                self.y,
                self.content_width(),
            );
        }
    }

    fn draw_table(&mut self, table: &TableContent) {
        if table.rows == 0 || table.cols == 0 {
            return;
        }
        let width = self.content_width();
        let col_width = width / table.cols as f64;
        let font = FontDescription::from_string("Sans 9");
        for row in 0..table.rows {
            let mut row_height: f64 = 18.0;
            for col in 0..table.cols {
                if let Some(cell) = table.cell(row, col) {
                    let layout = create_layout(&self.cr);
                    layout.set_width((col_width * FONT_SCALE as f64) as i32 - 8);
                    layout.set_wrap(WrapMode::WordChar);
                    layout.set_font_description(Some(&font));
                    layout.set_text(&cell.text());
                    row_height = row_height.max(layout.pixel_size().1 as f64 + 8.0);
                }
            }
            if self.y + row_height > self.content_bottom() && self.y > self.options.margin {
                self.show_page();
            }
            for col in 0..table.cols {
                let x = self.options.margin + col as f64 * col_width;
                let text = table
                    .cell(row, col)
                    .map(|cell| cell.text())
                    .unwrap_or_default();
                if row == 0 && table.header {
                    self.cr.set_source_rgb(0.92, 0.92, 0.92);
                    self.cr.rectangle(x, self.y, col_width, row_height);
                    let _ = self.cr.fill();
                }
                let layout = create_layout(&self.cr);
                layout.set_width((col_width * FONT_SCALE as f64) as i32 - 8);
                layout.set_wrap(WrapMode::WordChar);
                layout.set_font_description(Some(&font));
                layout.set_text(&text);
                self.cr.move_to(x + 4.0, self.y + 4.0);
                show_layout(&self.cr, &layout);
                self.cr.set_source_rgb(0.55, 0.55, 0.55);
                self.cr.rectangle(x, self.y, col_width, row_height);
                let _ = self.cr.stroke();
                self.cr.set_source_rgb(0.1, 0.1, 0.1);
            }
            self.y += row_height;
        }
        self.y += 6.0;
    }

    fn draw_image(&mut self, image: &ImageContent) {
        let height = image.height.unwrap_or(220.0).min(320.0);
        self.ensure_space(height + 24.0);
        let mut drawn = false;
        if let Some(path) = (self.resolver)(&image.asset)
            && let Ok(dynamic) = image::open(&path)
        {
            let rgba = dynamic.to_rgba8();
            let (w, h) = rgba.dimensions();
            if w > 0 && h > 0 {
                let scale = (self.content_width() / w as f64).min(height / h as f64);
                let draw_w = w as f64 * scale;
                let draw_h = h as f64 * scale;
                if let Ok(mut surface) = ImageSurface::create(Format::ARgb32, w as i32, h as i32) {
                    if let Ok(mut data) = surface.data() {
                        for (index, pixel) in rgba.pixels().enumerate() {
                            let offset = index * 4;
                            data[offset] = pixel[2];
                            data[offset + 1] = pixel[1];
                            data[offset + 2] = pixel[0];
                            data[offset + 3] = pixel[3];
                        }
                    }
                    let _ = self.cr.save();
                    self.cr.translate(self.options.margin, self.y);
                    self.cr.scale(draw_w / w as f64, draw_h / h as f64);
                    self.cr.set_source_surface(&surface, 0.0, 0.0).ok();
                    self.cr.rectangle(0.0, 0.0, w as f64, h as f64);
                    let _ = self.cr.fill();
                    let _ = self.cr.restore();
                    self.y += draw_h + 6.0;
                    drawn = true;
                }
            }
        }
        if !drawn {
            self.cr.set_source_rgb(0.9, 0.9, 0.9);
            self.cr
                .rectangle(self.options.margin, self.y, self.content_width(), height);
            let _ = self.cr.fill();
            let label = if image.caption.is_empty() {
                format!("[imagen {}]", &image.asset[..8.min(image.asset.len())])
            } else {
                image.caption.clone()
            };
            let layout = create_layout(&self.cr);
            layout.set_font_description(Some(&FontDescription::from_string("Sans 10")));
            layout.set_text(&label);
            self.cr.move_to(self.options.margin + 8.0, self.y + 8.0);
            show_layout(&self.cr, &layout);
            self.y += height + 6.0;
        }
        if !image.caption.is_empty() && drawn {
            let layout = create_layout(&self.cr);
            layout.set_font_description(Some(&FontDescription::from_string("Sans Italic 9")));
            layout.set_text(&image.caption);
            self.cr.set_source_rgb(0.4, 0.4, 0.4);
            self.cr.move_to(self.options.margin, self.y);
            show_layout(&self.cr, &layout);
            self.cr.set_source_rgb(0.1, 0.1, 0.1);
            self.y += 14.0;
        }
    }

    fn draw_media_card(&mut self, icon: &str, title: &str, detail: &str) {
        self.ensure_space(56.0);
        self.cr.set_source_rgb(0.95, 0.95, 0.95);
        self.cr
            .rectangle(self.options.margin, self.y, self.content_width(), 44.0);
        let _ = self.cr.fill();
        let layout = create_layout(&self.cr);
        layout.set_font_description(Some(&FontDescription::from_string("Sans 10")));
        layout.set_text(&format!("{icon}  {title}  {detail}"));
        self.cr.move_to(self.options.margin + 8.0, self.y + 14.0);
        show_layout(&self.cr, &layout);
        self.y += 52.0;
    }

    fn draw_web_card(&mut self, web: &WebContent) {
        self.ensure_space(70.0);
        self.cr.set_source_rgb(0.94, 0.97, 1.0);
        self.cr
            .rectangle(self.options.margin, self.y, self.content_width(), 58.0);
        let _ = self.cr.fill();
        let title = if web.title.is_empty() {
            web.url.clone()
        } else {
            web.title.clone()
        };
        let layout = create_layout(&self.cr);
        layout.set_width((self.content_width() * FONT_SCALE as f64) as i32 - 16);
        layout.set_wrap(WrapMode::WordChar);
        layout.set_font_description(Some(&FontDescription::from_string("Sans Bold 10")));
        layout.set_text(&title);
        self.cr.move_to(self.options.margin + 8.0, self.y + 8.0);
        show_layout(&self.cr, &layout);
        let layout = create_layout(&self.cr);
        layout.set_width((self.content_width() * FONT_SCALE as f64) as i32 - 16);
        layout.set_wrap(WrapMode::WordChar);
        layout.set_font_description(Some(&FontDescription::from_string("Sans 9")));
        layout.set_text(&format!("{} · {}", web.url, web.description));
        self.cr.move_to(self.options.margin + 8.0, self.y + 30.0);
        show_layout(&self.cr, &layout);
        self.y += 66.0;
    }

    fn draw_drawing(&mut self, drawing: &DrawingContent) {
        let height = 260.0;
        self.ensure_space(height + 8.0);
        let width = self.content_width();
        self.cr.set_source_rgb(0.98, 0.98, 0.98);
        self.cr
            .rectangle(self.options.margin, self.y, width, height);
        let _ = self.cr.fill();
        let sx = width / drawing.width.max(1.0);
        let sy = height / drawing.height.max(1.0);
        for stroke in &drawing.strokes {
            if stroke.points.len() < 2 {
                continue;
            }
            let (r, g, b) = parse_color(&stroke.color);
            self.cr.set_source_rgba(r, g, b, 1.0);
            self.cr.set_line_width(stroke.width.max(0.5));
            for (index, point) in stroke.points.iter().enumerate() {
                if point.len() < 2 {
                    continue;
                }
                let x = self.options.margin + point[0] * sx;
                let y = self.y + point[1] * sy;
                if index == 0 {
                    self.cr.move_to(x, y);
                } else {
                    self.cr.line_to(x, y);
                }
            }
            let _ = self.cr.stroke();
        }
        self.cr.set_source_rgb(0.1, 0.1, 0.1);
        self.y += height + 8.0;
    }

    fn draw_plugin(&mut self, plugin: &PluginContent) {
        self.ensure_space(70.0);
        self.cr.set_source_rgb(0.97, 0.97, 0.97);
        self.cr
            .rectangle(self.options.margin, self.y, self.content_width(), 58.0);
        let _ = self.cr.fill();
        let layout = create_layout(&self.cr);
        layout.set_font_description(Some(&FontDescription::from_string("Sans Bold 10")));
        layout.set_text(&format!("Plugin {} · {}", plugin.plugin, plugin.block));
        self.cr.move_to(self.options.margin + 8.0, self.y + 10.0);
        show_layout(&self.cr, &layout);
        let layout = create_layout(&self.cr);
        layout.set_width((self.content_width() * FONT_SCALE as f64) as i32 - 16);
        layout.set_wrap(WrapMode::WordChar);
        layout.set_font_description(Some(&FontDescription::from_string("Sans 8")));
        layout.set_text(&plugin.state.to_string());
        self.cr.move_to(self.options.margin + 8.0, self.y + 28.0);
        show_layout(&self.cr, &layout);
        self.y += 66.0;
    }

    fn draw_block(&mut self, block: &Block) {
        match &block.content {
            BlockContent::Text(text) => self.draw_text(text),
            BlockContent::Table(table) => self.draw_table(table),
            BlockContent::Image(image) => self.draw_image(image),
            BlockContent::File(file) => self.draw_media_card(
                "📎",
                &file.name,
                &format!("{} · {} bytes", file.mime, file.size),
            ),
            BlockContent::Audio(audio) => {
                self.draw_media_card("🔊", &audio.title, &format!("{} ms", audio.duration_ms))
            }
            BlockContent::Video(video) => {
                self.draw_media_card("🎬", &video.title, &format!("{} ms", video.duration_ms))
            }
            BlockContent::Web(web) => self.draw_web_card(web),
            BlockContent::Drawing(drawing) => self.draw_drawing(drawing),
            BlockContent::Plugin(plugin) => self.draw_plugin(plugin),
            BlockContent::Raw(_) => {
                self.draw_media_card("⚠", &block.kind, "bloque no compatible conservado");
            }
        }
    }

    fn draw_canvas_pages(&mut self, note: &Note) {
        let free: Vec<(&Block, f64, f64, f64, f64)> = note
            .blocks
            .iter()
            .filter_map(|block| {
                block
                    .placement
                    .rect()
                    .map(|(x, y, w, h, _z)| (block, x, y, w, h))
            })
            .collect();
        if free.is_empty() {
            return;
        }
        let min_x = free
            .iter()
            .map(|(_, x, _, _, _)| *x)
            .fold(f64::INFINITY, f64::min);
        let min_y = free
            .iter()
            .map(|(_, _, y, _, _)| *y)
            .fold(f64::INFINITY, f64::min);
        let max_x = free
            .iter()
            .map(|(_, x, _, w, _)| x + w)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = free
            .iter()
            .map(|(_, _, y, _, h)| y + h)
            .fold(f64::NEG_INFINITY, f64::max);
        let width = (max_x - min_x).max(1.0);
        let height = (max_y - min_y).max(1.0);
        let content_w = self.content_width();
        let content_h = self.content_bottom() - self.options.margin - 24.0;
        let scale = (content_w / width).min(content_h / height).min(1.0);
        if self.y > self.options.margin + 80.0 {
            self.show_page();
        }
        let layout = create_layout(&self.cr);
        layout.set_font_description(Some(&FontDescription::from_string("Sans Bold 16")));
        layout.set_text("Lienzo");
        self.cr.move_to(self.options.margin, self.y);
        show_layout(&self.cr, &layout);
        self.y += 26.0;
        let origin_x = self.options.margin;
        let origin_y = self.y;
        for (block, x, y, w, h) in free {
            let bx = origin_x + (x - min_x) * scale;
            let by = origin_y + (y - min_y) * scale;
            self.cr.set_source_rgb(0.9, 0.9, 0.9);
            self.cr.rectangle(bx, by, w * scale, h * scale);
            let _ = self.cr.stroke();
            let label = create_layout(&self.cr);
            label.set_width(((w * scale).max(20.0) * FONT_SCALE as f64) as i32 - 4);
            label.set_wrap(WrapMode::WordChar);
            label.set_font_description(Some(&FontDescription::from_string("Sans 8")));
            let preview = block
                .plain_text()
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            label.set_text(&format!(
                "{} @ ({:.0}, {:.0}): {}",
                block.kind, x, y, preview
            ));
            self.cr.move_to(bx + 3.0, by + 3.0);
            show_layout(&self.cr, &label);
        }
        self.cr.set_source_rgb(0.1, 0.1, 0.1);
    }
}

fn parse_color(hex: &str) -> (f64, f64, f64) {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (0.1, 0.1, 0.1);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(26) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(26) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(26) as f64 / 255.0;
    (r, g, b)
}

pub fn write_pdf(path: &Path, note: &Note, options: &PdfOptions) -> Result<(), FolioError> {
    write_pdf_with_assets(path, note, options, &|_| None)
}

pub fn write_pdf_with_assets(
    path: &Path,
    note: &Note,
    options: &PdfOptions,
    resolver: &AssetResolver<'_>,
) -> Result<(), FolioError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let surface =
        PdfSurface::new(options.page_width, options.page_height, path).map_err(pdf_error)?;
    let _ = surface.set_metadata(PdfMetadata::Creator, "Folio");
    let _ = surface.set_metadata(PdfMetadata::Title, &note.display_title());
    // Fecha fija para que el golden no dependa del reloj.
    let _ = surface.set_metadata(PdfMetadata::CreateDate, "D:20261029090000Z");
    let _ = surface.set_metadata(PdfMetadata::ModDate, "D:20261029090000Z");
    let cr = Context::new(&surface).map_err(pdf_error)?;
    cr.set_source_rgb(0.1, 0.1, 0.1);
    let title = options
        .title
        .clone()
        .unwrap_or_else(|| note.display_title());
    let mut writer = PdfWriter {
        cr,
        surface,
        options: PdfOptions {
            title: Some(title.clone()),
            ..options.clone()
        },
        page: 1,
        y: options.margin,
        resolver,
    };
    let title_layout = create_layout(&writer.cr);
    title_layout.set_width((writer.content_width() * FONT_SCALE as f64) as i32);
    title_layout.set_wrap(WrapMode::WordChar);
    title_layout.set_font_description(Some(&FontDescription::from_string("Sans Bold 24")));
    title_layout.set_text(&title);
    writer.y += title_layout.pixel_size().1 as f64 + 10.0;
    writer.cr.move_to(options.margin, options.margin);
    show_layout(&writer.cr, &title_layout);

    for block in &note.blocks {
        if block.placement.is_free() {
            continue;
        }
        writer.draw_block(block);
    }
    if options.include_canvas {
        writer.draw_canvas_pages(note);
    }
    writer.footer();
    writer.surface.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_core::FixedClock;
    use folio_core::model::{Paragraph, TableContent};

    fn note() -> Note {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let mut note = Note::new("Informe", &clock);
        note.blocks.push(Block::text(vec![
            Paragraph::styled("h1", "Informe de prueba"),
            Paragraph::body(
                "Texto normal con una línea larga para probar el paginado del documento.",
            ),
            Paragraph::styled("quote", "Una cita"),
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

    #[test]
    fn writes_a_pdf_with_pages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("informe.pdf");
        write_pdf(&path, &note(), &PdfOptions::default()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF-1."));
        assert!(bytes.len() > 1000);
        assert!(bytes.windows(5).any(|window| window == b"%PDF-"));
        if let Ok(info) = std::process::Command::new("pdfinfo").arg(&path).output() {
            let text = String::from_utf8_lossy(&info.stdout);
            assert!(text.contains("Pages:"));
        }
    }

    #[test]
    fn canvas_free_blocks_get_a_page() {
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
        assert!(std::fs::metadata(path).unwrap().len() > 1000);
    }
}
