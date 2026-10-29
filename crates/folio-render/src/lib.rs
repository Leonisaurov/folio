//! Exportadores Markdown/HTML/PDF e importador Markdown, sin dependencias de GTK.

pub mod pdf;

use folio_core::{
    Block, BlockContent, Clock, FolioError, Note,
    model::{
        DrawingContent, ImageContent, NoteLayout, Paragraph, PluginContent, Span, TableContent,
    },
    text::text_block,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Prefijo de URL/carpeta para assets. Por defecto `assets/`.
    pub asset_prefix: String,
    /// Incluye comentarios con las posiciones del canvas.
    pub include_canvas_positions: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            asset_prefix: "assets/".into(),
            include_canvas_positions: true,
        }
    }
}

fn asset_url(prefix: &str, hash: &str) -> String {
    format!("{prefix}{hash}")
}

fn markdown_spans(spans: &[Span]) -> String {
    let mut out = String::new();
    for span in spans {
        let mut text = span.text.clone();
        for mark in &span.marks {
            text = match mark.as_str() {
                "bold" => format!("**{text}**"),
                "italic" => format!("*{text}*"),
                "strikethrough" => format!("~~{text}~~"),
                "code" => format!("`{text}`"),
                "underline" => format!("<u>{text}</u>"),
                "highlight" => format!("=={text}=="),
                _ => text,
            };
        }
        if let Some(link) = &span.link {
            text = format!("[{text}]({link})");
        }
        out.push_str(&text);
    }
    out
}

fn markdown_paragraph(paragraph: &Paragraph) -> String {
    let text = markdown_spans(&paragraph.spans);
    match paragraph.style.as_str() {
        "h1" => format!("# {text}\n"),
        "h2" => format!("## {text}\n"),
        "h3" => format!("### {text}\n"),
        "quote" => format!("> {text}\n"),
        "list_item" => format!("- {text}\n"),
        "checklist_item" => {
            let checked = if paragraph.checked.unwrap_or(false) {
                "x"
            } else {
                " "
            };
            format!("- [{checked}] {text}\n")
        }
        "code" => format!(
            "```{}\n{}\n```\n",
            paragraph.lang.as_deref().unwrap_or(""),
            text
        ),
        _ => format!("{text}\n"),
    }
}

pub fn to_markdown(note: &Note, options: &ExportOptions) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", note.display_title()));
    if !note.meta.tags.is_empty() {
        out.push_str(&format!(
            "{} \n\n",
            note.meta
                .tags
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    for block in &note.blocks {
        if options.include_canvas_positions
            && let Some((x, y, w, h, z)) = block.placement.rect()
        {
            out.push_str(&format!(
                "<!-- folio:free x={x:.1} y={y:.1} w={w:.1} h={h:.1} z={z} -->\n"
            ));
        }
        match &block.content {
            BlockContent::Text(content) => {
                let mut previous_code = false;
                for paragraph in &content.paragraphs {
                    let is_code = paragraph.style == "code";
                    if previous_code && is_code {
                        out.push('\n');
                    }
                    out.push_str(&markdown_paragraph(paragraph));
                    out.push('\n');
                    previous_code = is_code;
                }
            }
            BlockContent::Table(table) => out.push_str(&table_markdown(table)),
            BlockContent::Image(image) => {
                out.push_str(&format!(
                    "![{}]({})\n",
                    text_or(&image.alt, &image.caption),
                    image_asset_path(image, &options.asset_prefix)
                ));
                if !image.caption.is_empty() {
                    out.push_str(&format!("\n*{}*\n", image.caption));
                }
            }
            BlockContent::File(file) => out.push_str(&format!(
                "[{}]({})\n",
                file.name,
                asset_url(&options.asset_prefix, &file.asset)
            )),
            BlockContent::Audio(audio) => out.push_str(&format!(
                "[🔊 {}]({})\n",
                audio.title,
                asset_url(&options.asset_prefix, &audio.asset)
            )),
            BlockContent::Video(video) => out.push_str(&format!(
                "[🎬 {}]({})\n",
                video.title,
                asset_url(&options.asset_prefix, &video.asset)
            )),
            BlockContent::Web(web) => {
                let title = text_or(&web.title, &web.url);
                out.push_str(&format!("[{}]({})\n", title, web.url));
                if !web.description.is_empty() {
                    out.push_str(&format!("\n> {}\n", web.description));
                }
            }
            BlockContent::Drawing(drawing) => out.push_str(&format!(
                "<!-- dibujo vectorial: {}x{}, {} trazos; exporta HTML para verlo -->\n",
                drawing.width,
                drawing.height,
                drawing.strokes.len()
            )),
            BlockContent::Plugin(plugin) => out.push_str(&format!(
                "<!-- plugin:{} state={} -->\n",
                plugin.plugin, plugin.state
            )),
            BlockContent::Raw(value) => {
                out.push_str(&format!("<!-- bloque no compatible: {value} -->\n"))
            }
        }
        out.push('\n');
    }
    out
}

fn image_asset_path(image: &ImageContent, prefix: &str) -> String {
    if let Some(path) = &image.external_path {
        path.clone()
    } else {
        asset_url(prefix, &image.asset)
    }
}

fn text_or(primary: &str, fallback: &str) -> String {
    if primary.is_empty() {
        fallback.to_string()
    } else {
        primary.to_string()
    }
}

fn table_markdown(table: &TableContent) -> String {
    let mut out = String::new();
    if table.rows == 0 || table.cols == 0 {
        return out;
    }
    let cell = |row: usize, col: usize| {
        table
            .cell(row, col)
            .map(|cell| {
                cell.paragraphs
                    .iter()
                    .map(|paragraph| markdown_spans(&paragraph.spans))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace('|', "\\|")
                    .replace('\n', " ")
            })
            .unwrap_or_default()
    };
    let mut row = String::new();
    for col in 0..table.cols {
        row.push_str(&format!("| {} ", cell(0, col)));
    }
    out.push_str(&row);
    out.push_str("|\n");
    out.push('|');
    for _ in 0..table.cols {
        out.push_str(" --- |");
    }
    out.push('\n');
    for index in 1..table.rows {
        let mut row = String::new();
        for col in 0..table.cols {
            row.push_str(&format!("| {} ", cell(index, col)));
        }
        out.push_str(&row);
        out.push_str("|\n");
    }
    out
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn html_spans(spans: &[Span]) -> String {
    let mut out = String::new();
    for span in spans {
        let mut text = escape_html(&span.text);
        for mark in &span.marks {
            text = match mark.as_str() {
                "bold" => format!("<strong>{text}</strong>"),
                "italic" => format!("<em>{text}</em>"),
                "strikethrough" => format!("<del>{text}</del>"),
                "code" => format!("<code>{text}</code>"),
                "underline" => format!("<u>{text}</u>"),
                "highlight" => format!("<mark>{text}</mark>"),
                "superscript" => format!("<sup>{text}</sup>"),
                "subscript" => format!("<sub>{text}</sub>"),
                _ => text,
            };
        }
        if let Some(link) = &span.link {
            text = format!("<a href=\"{}\">{text}</a>", escape_html(link));
        }
        out.push_str(&text);
    }
    out
}

fn html_paragraph(paragraph: &Paragraph) -> String {
    let content = html_spans(&paragraph.spans);
    let style = format!(" class=\"style-{}\"", escape_html(&paragraph.style));
    match paragraph.style.as_str() {
        "h1" => format!("<h1{style}>{content}</h1>"),
        "h2" => format!("<h2{style}>{content}</h2>"),
        "h3" => format!("<h3{style}>{content}</h3>"),
        "quote" => format!("<blockquote{style}>{content}</blockquote>"),
        "list_item" => format!(
            "<div class=\"list-item\"{style}><span class=\"bullet\">•</span>{content}</div>"
        ),
        "checklist_item" => {
            let checked = if paragraph.checked.unwrap_or(false) {
                "checked"
            } else {
                ""
            };
            format!(
                "<div class=\"list-item checklist\"{style}><input type=\"checkbox\" disabled {checked}>{content}</div>"
            )
        }
        "code" => format!("<pre class=\"style-code\"><code>{content}</code></pre>"),
        _ => format!("<p{style}>{content}</p>"),
    }
}

pub fn to_html(note: &Note, options: &ExportOptions) -> String {
    let mut body = String::new();
    let layout: &NoteLayout = &note.layout;
    body.push_str(&format!(
        "<article class=\"folio-note\" style=\"--flow-width:{}px\">\n",
        layout.flow_width
    ));
    body.push_str(&format!(
        "<h1 class=\"note-title\">{}</h1>\n",
        escape_html(&note.display_title())
    ));
    if !note.meta.tags.is_empty() {
        body.push_str("<p class=\"tags\">");
        for tag in &note.meta.tags {
            body.push_str(&format!(
                "<span class=\"tag\">#{}</span> ",
                escape_html(tag)
            ));
        }
        body.push_str("</p>\n");
    }
    for block in &note.blocks {
        let style = if options.include_canvas_positions {
            let (x, y, w, h, z) =
                block
                    .placement
                    .rect()
                    .unwrap_or((0.0, 0.0, layout.flow_width, 0.0, 0));
            if block.placement.is_free() {
                format!(
                    " style=\"position:absolute;left:{x:.1}px;top:{y:.1}px;width:{w:.1}px;min-height:{h:.1}px;z-index:{z}\""
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        body.push_str(&format!(
            "<section class=\"block block-{}\"{style}>\n",
            escape_html(&block.kind)
        ));
        match &block.content {
            BlockContent::Text(content) => {
                for paragraph in &content.paragraphs {
                    body.push_str(&html_paragraph(paragraph));
                    body.push('\n');
                }
            }
            BlockContent::Table(table) => {
                body.push_str(&table_html(table));
            }
            BlockContent::Image(image) => {
                body.push_str(&format!(
                    "<figure><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>\n",
                    escape_html(&image_asset_path(image, &options.asset_prefix)),
                    escape_html(&image.alt),
                    escape_html(&image.caption)
                ));
            }
            BlockContent::File(file) => body.push_str(&format!(
                "<a class=\"file\" href=\"{}\">{}</a>\n",
                escape_html(&asset_url(&options.asset_prefix, &file.asset)),
                escape_html(&file.name)
            )),
            BlockContent::Audio(audio) => body.push_str(&format!(
                "<audio controls src=\"{}\"></audio>\n",
                escape_html(&asset_url(&options.asset_prefix, &audio.asset))
            )),
            BlockContent::Video(video) => body.push_str(&format!(
                "<video controls src=\"{}\"></video>\n",
                escape_html(&asset_url(&options.asset_prefix, &video.asset))
            )),
            BlockContent::Web(web) => body.push_str(&format!(
                "<div class=\"web-card\"><a href=\"{}\">{}</a><p>{}</p></div>\n",
                escape_html(&web.url),
                escape_html(&text_or(&web.title, &web.url)),
                escape_html(&web.description)
            )),
            BlockContent::Drawing(drawing) => body.push_str(&drawing_svg(drawing)),
            BlockContent::Plugin(plugin) => body.push_str(&plugin_html(plugin)),
            BlockContent::Raw(_value) => body.push_str(&format!(
                "<div class=\"unsupported\">Bloque no compatible: {}</div>\n",
                escape_html(&block.kind)
            )),
        }
        body.push_str("</section>\n");
    }
    body.push_str("</article>\n");
    format!(
        "<!doctype html>\n<html lang=\"es\"><head><meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>\n{css}\n</style></head><body>\n{body}</body></html>\n",
        title = escape_html(&note.display_title()),
        css = CSS,
    )
}

const CSS: &str = r#":root { color-scheme: light dark; }
body { margin: 0; padding: 32px; font-family: system-ui, sans-serif; background: #faf9f6; color: #1c1c1c; }
.folio-note { position: relative; max-width: var(--flow-width); margin: 0 auto; }
.note-title { font-size: 2.2rem; line-height: 1.15; }
.block { margin: 12px 0; }
.block-text p { line-height: 1.55; margin: 0.6em 0; }
.style-h1, .style-h2, .style-h3 { line-height: 1.2; margin: 1.2em 0 0.4em; }
.style-quote { border-left: 3px solid #3584e4; margin-left: 0; padding-left: 1em; color: #555; }
.style-code { background: #eee; padding: 12px; border-radius: 6px; overflow-x: auto; }
.list-item { display: flex; gap: 0.5em; line-height: 1.5; }
.bullet { color: #3584e4; }
table { border-collapse: collapse; width: 100%; }
th, td { border: 1px solid #ccc; padding: 6px 8px; text-align: left; }
img, video { max-width: 100%; height: auto; border-radius: 6px; }
figure { margin: 0; }
figcaption, .tags { color: #666; font-size: 0.9em; }
.web-card { border: 1px solid #ccc; border-radius: 8px; padding: 12px; }
.unsupported { border: 1px dashed #c00; padding: 8px; color: #900; }
@media (prefers-color-scheme: dark) {
  body { background: #1c1c1c; color: #eee; }
  .style-code, .web-card { background: #2a2a2a; }
  th, td { border-color: #444; }
  .style-quote { color: #bbb; }
}
"#;

fn table_html(table: &TableContent) -> String {
    let mut out = String::from("<table>\n");
    for row in 0..table.rows {
        out.push_str("<tr>");
        for col in 0..table.cols {
            let tag = if row == 0 && table.header { "th" } else { "td" };
            let text = table
                .cell(row, col)
                .map(|cell| {
                    cell.paragraphs
                        .iter()
                        .map(|paragraph| html_spans(&paragraph.spans))
                        .collect::<Vec<_>>()
                        .join("<br>")
                })
                .unwrap_or_default();
            out.push_str(&format!("<{tag}>{text}</{tag}>"));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n");
    out
}

fn drawing_svg(drawing: &DrawingContent) -> String {
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">\n",
        drawing.width, drawing.height, drawing.width, drawing.height
    );
    for stroke in &drawing.strokes {
        if stroke.points.len() < 2 {
            continue;
        }
        let points: Vec<String> = stroke
            .points
            .iter()
            .filter_map(|point| {
                if point.len() >= 2 {
                    Some(format!("{},{}", point[0], point[1]))
                } else {
                    None
                }
            })
            .collect();
        out.push_str(&format!(
            "<polyline fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" points=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\n",
            escape_html(&stroke.color),
            stroke.width,
            points.join(" ")
        ));
    }
    out.push_str("</svg>\n");
    out
}

fn plugin_html(plugin: &PluginContent) -> String {
    format!(
        "<div class=\"plugin-placeholder\" data-plugin=\"{}\">Plugin {}: {}</div>\n",
        escape_html(&plugin.plugin),
        escape_html(&plugin.plugin),
        escape_html(&plugin.block)
    )
}

fn heading_style(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => "h1",
        HeadingLevel::H2 => "h2",
        HeadingLevel::H3 => "h3",
        _ => "body",
    }
}

/// Importa Markdown saneado. El HTML crudo nunca se ejecuta ni se conserva.
pub fn import_markdown(text: &str, title: &str, clock: &dyn Clock) -> Note {
    let mut note = Note::new(title, clock);
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let parser = Parser::new_ext(text, options);
    let mut spans: Vec<Span> = Vec::new();
    let mut style = "body".to_string();
    let mut list_stack: Vec<bool> = Vec::new();
    let mut code = String::new();
    let mut in_code = false;
    let mut table: Option<TableContent> = None;
    let mut cell_text = String::new();
    let mut cell_row = 0usize;
    let mut cell_col = 0usize;
    let mut in_table_head = false;
    let mut marks: Vec<String> = Vec::new();
    let mut link: Option<String> = None;

    let flush = |spans: &mut Vec<Span>, style: &str, checked: Option<bool>| {
        if spans.is_empty() {
            return None;
        }
        let mut paragraph = Paragraph {
            style: style.to_string(),
            spans: std::mem::take(spans),
            checked,
            ..Paragraph::body("")
        };
        paragraph.checked = checked;
        Some(paragraph)
    };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => style = heading_style(level).to_string(),
            Event::End(TagEnd::Heading(_)) => {
                if let Some(paragraph) = flush(&mut spans, &style, None) {
                    note.blocks.push(Block::text(vec![paragraph]));
                }
                style = "body".into();
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if let Some(paragraph) = flush(&mut spans, &style, None) {
                    note.blocks.push(Block::text(vec![paragraph]));
                }
            }
            Event::Start(Tag::BlockQuote(_)) => style = "quote".into(),
            Event::End(TagEnd::BlockQuote(_)) => style = "body".into(),
            Event::Start(Tag::List(start)) => list_stack.push(start.is_some()),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let _ordered = list_stack.last().copied().unwrap_or(false);
                style = "list_item".into();
            }
            Event::End(TagEnd::Item) => {
                if let Some(paragraph) = flush(&mut spans, &style, None) {
                    note.blocks.push(Block::text(vec![paragraph]));
                }
                style = "body".into();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code = true;
                code.clear();
                style = "code".into();
                if let CodeBlockKind::Fenced(info) = kind {
                    let _ = info;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                let mut paragraph = Paragraph::styled("code", code.trim_end());
                if let Some(BlockContent::Text(existing)) =
                    note.blocks.last_mut().map(|b| &mut b.content)
                {
                    let _ = existing;
                }
                paragraph.lang = None;
                note.blocks.push(Block::text(vec![paragraph]));
                style = "body".into();
            }
            Event::Start(Tag::Table(_)) => {
                table = Some(TableContent::new(0, 0));
                cell_row = 0;
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
            }
            Event::End(TagEnd::TableHead) => {
                in_table_head = false;
                cell_row = 1;
            }
            Event::Start(Tag::TableRow) => {
                cell_col = 0;
            }
            Event::End(TagEnd::TableRow) => {
                if !in_table_head {
                    cell_row += 1;
                }
            }
            Event::Start(Tag::TableCell) => cell_text.clear(),
            Event::End(TagEnd::TableCell) => {
                if let Some(table) = &mut table {
                    if table.rows <= cell_row {
                        table.rows = cell_row + 1;
                    }
                    if table.cols <= cell_col {
                        table.cols = cell_col + 1;
                        table.widths.resize(table.cols, 180.0);
                    }
                    table.set_cell(cell_row, cell_col, &cell_text);
                    cell_col += 1;
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(table) = table.take()
                    && table.rows > 0
                    && table.cols > 0
                {
                    note.blocks
                        .push(Block::new("table", BlockContent::Table(table)));
                }
            }
            Event::Start(Tag::Emphasis) => marks.push("italic".into()),
            Event::End(TagEnd::Emphasis) => {
                marks.pop();
            }
            Event::Start(Tag::Strong) => marks.push("bold".into()),
            Event::End(TagEnd::Strong) => {
                marks.pop();
            }
            Event::Start(Tag::Strikethrough) => marks.push("strikethrough".into()),
            Event::End(TagEnd::Strikethrough) => {
                marks.pop();
            }
            Event::Start(Tag::Link { dest_url, .. }) => link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => link = None,
            Event::Start(Tag::Image { dest_url, .. }) => {
                let mut image = ImageContent {
                    external_path: Some(dest_url.to_string()),
                    ..Default::default()
                };
                image.caption = String::new();
                // Sin acceso al store aquí: se conserva como enlace externo y el
                // llamador puede importarlo y sustituir el asset.
                spans.push(Span::plain(format!("![imagen]({dest_url})")));
                let _ = image;
            }
            Event::Text(text) => {
                if in_code {
                    code.push_str(&text);
                } else if table.is_some() {
                    cell_text.push_str(&text);
                } else if let Some(link) = &link {
                    spans.push(Span {
                        text: text.to_string(),
                        marks: marks.clone(),
                        link: Some(link.clone()),
                        color: None,
                        extra: Default::default(),
                    });
                } else if marks.is_empty() {
                    if let Some(last) = spans.last_mut()
                        && last.link.is_none()
                        && last.marks.is_empty()
                    {
                        last.text.push_str(&text);
                        continue;
                    }
                    spans.push(Span::plain(text.to_string()));
                } else {
                    spans.push(Span {
                        text: text.to_string(),
                        marks: marks.clone(),
                        link: None,
                        color: None,
                        extra: Default::default(),
                    });
                }
            }
            Event::Code(text) => {
                if table.is_some() {
                    cell_text.push_str(&text);
                } else {
                    spans.push(Span::marked(text.to_string(), &["code"]));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if table.is_some() {
                    cell_text.push('\n');
                } else if let Some(last) = spans.last_mut() {
                    last.text.push('\n');
                }
            }
            Event::TaskListMarker(checked) => {
                let mut paragraph = Paragraph::styled("checklist_item", "");
                paragraph.checked = Some(checked);
                let _ = paragraph;
            }
            Event::Html(_) | Event::InlineHtml(_) => {
                // Se ignora por seguridad.
            }
            Event::Rule => {
                note.blocks.push(text_block("—"));
            }
            _ => {}
        }
    }
    // Cualquier texto suelto al final.
    if let Some(paragraph) = flush(&mut spans, "body", None) {
        note.blocks.push(Block::text(vec![paragraph]));
    }
    note.touch(clock);
    note
}

pub fn note_from_plain(text: &str, title: &str, clock: &dyn Clock) -> Note {
    let mut note = Note::new(title, clock);
    note.blocks.push(text_block(text));
    note.touch(clock);
    note
}

pub fn write_markdown(
    path: &std::path::Path,
    note: &Note,
    options: &ExportOptions,
) -> Result<(), FolioError> {
    std::fs::write(path, to_markdown(note, options))?;
    Ok(())
}

pub fn write_html(
    path: &std::path::Path,
    note: &Note,
    options: &ExportOptions,
) -> Result<(), FolioError> {
    std::fs::write(path, to_html(note, options))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_core::model::{FileContent, Paragraph, TableContent, TextContent, WebContent};

    fn sample() -> Note {
        let clock = folio_core::FixedClock::at("2026-10-29T09:00:00Z");
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
        note.blocks.push(Block::new(
            "drawing",
            BlockContent::Drawing(DrawingContent {
                strokes: vec![folio_core::model::Stroke {
                    points: vec![vec![0.0, 0.0], vec![10.0, 10.0]],
                    ..Default::default()
                }],
                ..Default::default()
            }),
        ));
        note.blocks.push(Block::new(
            "file",
            BlockContent::File(FileContent {
                asset: "b".repeat(64),
                name: "contrato.pdf".into(),
                ..Default::default()
            }),
        ));
        note
    }

    #[test]
    fn markdown_contains_all_representable_blocks() {
        let markdown = to_markdown(&sample(), &ExportOptions::default());
        assert!(markdown.contains("# Jardín"));
        assert!(markdown.contains("## Sección"));
        assert!(markdown.contains("**mundo**"));
        assert!(markdown.contains("[enlace](https://example.com)"));
        assert!(markdown.contains("> Una cita"));
        assert!(markdown.contains("- [x] hecho"));
        assert!(markdown.contains("| A | B |"));
        assert!(markdown.contains("![Croquis](assets/"));
        assert!(markdown.contains("[Artículo](https://example.com/articulo)"));
        assert!(markdown.contains("[contrato.pdf](assets/"));
        assert!(markdown.contains("dibujo vectorial"));
    }

    #[test]
    fn html_is_self_contained_and_has_no_scripts() {
        let html = to_html(&sample(), &ExportOptions::default());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<style>"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("javascript:"));
        assert!(html.contains("<strong>mundo</strong>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<svg"));
        assert!(html.contains("&lt;") || !html.contains("<script"));
    }

    #[test]
    fn html_escapes_text_and_links() {
        let clock = folio_core::FixedClock::at("2026-10-29T09:00:00Z");
        let mut note = Note::new("<script>alert(1)</script>", &clock);
        note.blocks
            .push(Block::text_plain("<img src=x onerror=alert(1)>"));
        let html = to_html(&note, &ExportOptions::default());
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;img"));
    }

    #[test]
    fn canvas_positions_are_represented() {
        let clock = folio_core::FixedClock::at("2026-10-29T09:00:00Z");
        let mut note = Note::new("canvas", &clock);
        let mut block = Block::text_plain("libre");
        block.placement = folio_core::Placement::free(120.0, 80.0, 300.0, 100.0);
        note.blocks.push(block);
        let markdown = to_markdown(&note, &ExportOptions::default());
        assert!(markdown.contains("<!-- folio:free x=120.0 y=80.0"));
        let html = to_html(&note, &ExportOptions::default());
        assert!(html.contains("position:absolute;left:120.0px;top:80.0px"));
    }

    #[test]
    fn import_markdown_builds_valid_note() {
        let clock = folio_core::FixedClock::at("2026-10-29T09:00:00Z");
        let text = "# Título\n\nHola **mundo** y *tú*.\n\n- uno\n- dos\n\n- [x] hecho\n\n```rust\nfn main() {}\n```\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n\n<script>alert(1)</script>\n";
        let note = import_markdown(text, "Importada", &clock);
        note.validate().unwrap();
        assert_eq!(note.blocks[0].plain_text(), "Título");
        let all = note.plain_text();
        assert!(all.contains("mundo"));
        assert!(all.contains("uno"));
        assert!(!all.contains("<script>"));
        assert!(note.blocks.iter().any(|block| block.kind == "table"));
        assert!(note.blocks.iter().any(|block| block.kind == "text"));
    }

    #[test]
    fn roundtrip_plain_text() {
        let clock = folio_core::FixedClock::at("2026-10-29T09:00:00Z");
        let note = note_from_plain("Una nota\ny dos líneas", "T", &clock);
        let markdown = to_markdown(&note, &ExportOptions::default());
        let imported = import_markdown(&markdown, "T", &clock);
        assert!(imported.plain_text().contains("Una nota"));
        assert!(imported.plain_text().contains("dos líneas"));
    }
}
