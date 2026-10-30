use std::rc::Rc;

use folio_core::model::{
    BlockContent, DrawingContent, Paragraph, PluginContent, Span, Stroke, TableContent, TextContent,
};
use folio_core::{Block, Placement};
use folio_plugin_sdk::{ChartKind, LabelStyle, UiNode};
use gtk::gdk;
use gtk::prelude::*;

use crate::state::estimate_height;
use crate::window::App;

pub const TEXT_TAGS: &[(&str, &str)] = &[
    ("bold", "Negrita"),
    ("italic", "Cursiva"),
    ("underline", "Subrayado"),
    ("strikethrough", "Tachado"),
    ("code", "Código"),
    ("highlight", "Resaltado"),
];

pub fn ensure_text_tags(buffer: &gtk::TextBuffer) {
    let table = buffer.tag_table();
    let add = |name: &str| -> gtk::TextTag {
        if let Some(tag) = table.lookup(name) {
            return tag;
        }
        let tag = gtk::TextTag::new(Some(name));
        table.add(&tag);
        tag
    };
    add("bold").set_weight(700);
    add("italic").set_style(gtk::pango::Style::Italic);
    add("underline").set_underline(gtk::pango::Underline::Single);
    add("strikethrough").set_strikethrough(true);
    add("code").set_family(Some("monospace"));
    let highlight = add("highlight");
    highlight.set_background(Some("#f9e27d"));
    highlight.set_foreground(Some("#1c1c1c"));
    let h1 = add("para-h1");
    h1.set_scale(2.0);
    h1.set_weight(700);
    h1.set_pixels_above_lines(10);
    h1.set_pixels_below_lines(6);
    let h2 = add("para-h2");
    h2.set_scale(1.5);
    h2.set_weight(700);
    h2.set_pixels_above_lines(8);
    let h3 = add("para-h3");
    h3.set_scale(1.25);
    h3.set_weight(700);
    let quote = add("para-quote");
    quote.set_style(gtk::pango::Style::Italic);
    quote.set_foreground(Some("#555555"));
    quote.set_left_margin(16);
    add("para-code").set_family(Some("monospace"));
    add("para-list").set_left_margin(18);
    add("para-checklist").set_left_margin(18);
    let _ = add("link");
}

fn apply_paragraph_tags(
    buffer: &gtk::TextBuffer,
    start_offset: i32,
    end_offset: i32,
    paragraph: &Paragraph,
) {
    let start = buffer.iter_at_offset(start_offset);
    let mut end = buffer.iter_at_offset(end_offset);
    if !end.ends_line() {
        end.forward_to_line_end();
        end.forward_char();
    }
    let tag = match paragraph.style.as_str() {
        "h1" => "para-h1",
        "h2" => "para-h2",
        "h3" => "para-h3",
        "quote" => "para-quote",
        "code" => "para-code",
        "list_item" => "para-list",
        "checklist_item" => "para-checklist",
        _ => "",
    };
    if !tag.is_empty() {
        buffer.apply_tag_by_name(tag, &start, &end);
    }
}

pub fn render_text_block(
    app: &Rc<App>,
    block: &Block,
    text: &TextContent,
    editable: bool,
) -> gtk::Widget {
    let view = gtk::TextView::new();
    view.set_wrap_mode(gtk::WrapMode::Word);
    view.set_left_margin(12);
    view.set_right_margin(12);
    view.set_top_margin(8);
    view.set_bottom_margin(8);
    view.set_editable(editable);
    view.set_cursor_visible(editable);
    view.set_monospace(false);
    view.add_css_class("folio-editor");
    if let Some(paragraph) = text.paragraphs.first()
        && (paragraph.style == "code")
    {
        view.set_monospace(true);
    }
    let buffer = view.buffer();
    ensure_text_tags(&buffer);
    app.suppress.set(true);
    let mut offset = 0i32;
    for (index, paragraph) in text.paragraphs.iter().enumerate() {
        let mut plain = paragraph.plain_text();
        if paragraph.style == "checklist_item" {
            plain = format!(
                "{} {plain}",
                if paragraph.checked.unwrap_or(false) {
                    "☑"
                } else {
                    "☐"
                }
            );
        }
        let start_offset = offset;
        let mut end = buffer.end_iter();
        buffer.insert(&mut end, &plain);
        let end_offset = offset + plain.chars().count() as i32;
        apply_paragraph_tags(&buffer, start_offset, end_offset, paragraph);
        let mut span_offset = start_offset;
        let prefix = if paragraph.style == "checklist_item" {
            2
        } else {
            0
        };
        for span in &paragraph.spans {
            let length = span.text.chars().count() as i32;
            let span_start = span_offset + prefix;
            let span_end = span_start + length;
            let iter = buffer.iter_at_offset(span_start);
            let mut end_iter = buffer.iter_at_offset(span_end);
            for mark in &span.marks {
                if buffer.tag_table().lookup(mark).is_some() {
                    buffer.apply_tag_by_name(mark, &iter, &end_iter);
                }
            }
            if let Some(link) = &span.link {
                if buffer.tag_table().lookup(&format!("link:{link}")).is_none() {
                    let tag = gtk::TextTag::new(Some(&format!("link:{link}")));
                    tag.set_underline(gtk::pango::Underline::Single);
                    tag.set_foreground(Some("#3584e4"));
                    buffer.tag_table().add(&tag);
                }
                buffer.apply_tag_by_name(&format!("link:{link}"), &iter, &end_iter);
            }
            span_offset = span_end;
            let _ = &mut end_iter;
        }
        if index + 1 < text.paragraphs.len() {
            let mut end = buffer.end_iter();
            buffer.insert(&mut end, "\n");
            offset = end_offset + 1;
        } else {
            offset = end_offset;
        }
    }
    app.suppress.set(false);

    let block_id = block.id.clone();
    let app_changed = app.clone();
    buffer.connect_changed(move |buffer| {
        if app_changed.suppress.get() {
            return;
        }
        let content = content_from_buffer(buffer);
        app_changed
            .state
            .borrow_mut()
            .update_text(&block_id, content);
        app_changed.schedule_save();
    });
    let app_focus = app.clone();
    let view_focus = view.clone();
    let focus = gtk::EventControllerFocus::new();
    focus.connect_enter(move |_| {
        *app_focus.focused.borrow_mut() = Some(view_focus.clone());
    });
    view.add_controller(focus);
    // La altura se ajusta al contenido para que el flujo sea natural.
    let app_resize = app.clone();
    let view_resize = view.clone();
    buffer.connect_changed(move |_| {
        let _ = &app_resize;
        let _ = &view_resize;
    });
    let frame = gtk::Frame::new(None);
    frame.add_css_class("folio-block");
    frame.set_child(Some(&view));
    frame.upcast()
}

/// Reconstruye el modelo de texto leyendo el buffer y sus tags.
pub fn content_from_buffer(buffer: &gtk::TextBuffer) -> TextContent {
    let line_count = buffer.line_count();
    let mut paragraphs = Vec::new();
    for line in 0..line_count {
        let Some(start) = buffer.iter_at_line(line) else {
            continue;
        };
        let mut end = start;
        end.forward_to_line_end();
        let raw = buffer.text(&start, &end, false).to_string();
        if raw.is_empty() && line + 1 == line_count {
            continue;
        }
        let tags: Vec<String> = start
            .tags()
            .into_iter()
            .filter_map(|tag| tag.name().map(|name| name.to_string()))
            .collect();
        let style = if tags.iter().any(|tag| tag == "para-h1") {
            "h1"
        } else if tags.iter().any(|tag| tag == "para-h2") {
            "h2"
        } else if tags.iter().any(|tag| tag == "para-h3") {
            "h3"
        } else if tags.iter().any(|tag| tag == "para-quote") {
            "quote"
        } else if tags.iter().any(|tag| tag == "para-code") {
            "code"
        } else if tags.iter().any(|tag| tag == "para-list") {
            "list_item"
        } else if tags.iter().any(|tag| tag == "para-checklist") {
            "checklist_item"
        } else {
            "body"
        };
        let mut checked = None;
        let mut text = raw;
        if style == "checklist_item" {
            if let Some(rest) = text.strip_prefix("☑ ") {
                checked = Some(true);
                text = rest.to_string();
            } else if let Some(rest) = text.strip_prefix("☐ ") {
                checked = Some(false);
                text = rest.to_string();
            }
        }
        let spans = spans_from_buffer(buffer, &start, &text, style == "checklist_item");
        let mut paragraph = Paragraph {
            style: style.into(),
            align: "left".into(),
            indent: 0,
            checked,
            lang: None,
            spans,
            extra: Default::default(),
        };
        if paragraph.spans.is_empty() {
            paragraph.spans.push(Span::plain(""));
        }
        paragraphs.push(paragraph);
    }
    if paragraphs.is_empty() {
        paragraphs.push(Paragraph::body(""));
    }
    TextContent {
        paragraphs,
        ..Default::default()
    }
}

fn spans_from_buffer(
    buffer: &gtk::TextBuffer,
    start: &gtk::TextIter,
    text: &str,
    strip_checkbox: bool,
) -> Vec<Span> {
    if text.is_empty() {
        return vec![Span::plain("")];
    }
    let base = start.offset();
    let chars: Vec<char> = text.chars().collect();
    let mut spans: Vec<Span> = Vec::new();
    let mut current: Option<Span> = None;
    let skip = if strip_checkbox { 2 } else { 0 };
    for (index, ch) in chars.iter().enumerate() {
        if index < skip {
            continue;
        }
        let iter = buffer.iter_at_offset(base + index as i32);
        let tags: Vec<String> = iter
            .tags()
            .into_iter()
            .filter_map(|tag| tag.name().map(|name| name.to_string()))
            .collect();
        let mut marks: Vec<String> = tags
            .iter()
            .filter(|tag| TEXT_TAGS.iter().any(|(name, _)| name == &tag.as_str()))
            .cloned()
            .collect();
        marks.sort();
        let link = tags
            .iter()
            .find_map(|tag| tag.strip_prefix("link:").map(str::to_string));
        let same = current
            .as_ref()
            .map(|span| span.marks == marks && span.link == link)
            .unwrap_or(false);
        if same {
            if let Some(span) = current.as_mut() {
                span.text.push(*ch);
            }
        } else {
            if let Some(span) = current.take() {
                spans.push(span);
            }
            current = Some(Span {
                text: ch.to_string(),
                marks,
                link,
                color: None,
                extra: Default::default(),
            });
        }
    }
    if let Some(span) = current {
        spans.push(span);
    }
    if spans.is_empty() {
        spans.push(Span::plain(""));
    }
    spans
}

pub fn block_body(app: &Rc<App>, block: &Block, editable: bool) -> gtk::Widget {
    match &block.content {
        BlockContent::Text(text) => render_text_block(app, block, text, editable),
        BlockContent::Table(table) => render_table(app, block, table, editable),
        BlockContent::Image(image) => render_image(app, image),
        BlockContent::File(file) => render_file(app, file),
        BlockContent::Audio(audio) => render_media(app, &audio.asset, &audio.title, false),
        BlockContent::Video(video) => render_media(app, &video.asset, &video.title, true),
        BlockContent::Web(web) => render_web(app, block, web),
        BlockContent::Drawing(drawing) => render_drawing(app, block, drawing, editable),
        BlockContent::Plugin(plugin) => render_plugin(app, block, plugin),
        BlockContent::Raw(value) => {
            let label = gtk::Label::new(Some(&format!(
                "Bloque no compatible ({}): {}",
                block.kind, value
            )));
            label.set_wrap(true);
            label.add_css_class("unsupported");
            label.upcast()
        }
    }
}

pub fn block_widget(app: &Rc<App>, block: &Block, editable: bool) -> gtk::Widget {
    let body = block_body(app, block, editable);
    let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
    wrapper.add_css_class("folio-block-wrap");
    if editable {
        wrapper.append(&block_header(app, block));
    }
    wrapper.append(&body);
    wrapper.upcast()
}

fn block_header(app: &Rc<App>, block: &Block) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    row.add_css_class("block-header");
    let kind = gtk::Label::new(Some(&kind_label(&block.kind)));
    kind.add_css_class("block-kind");
    kind.set_hexpand(true);
    kind.set_xalign(0.0);
    row.append(&kind);
    if block.placement.is_free() {
        let badge = gtk::Label::new(Some("anclado"));
        badge.add_css_class("badge");
        row.append(&badge);
    }
    let id = block.id.clone();
    let up = icon_button("go-up-symbolic", "Subir bloque");
    {
        let app = app.clone();
        let id = id.clone();
        up.connect_clicked(move |_| {
            app.state.borrow_mut().move_block(&id, -1);
            app.refresh_editor();
            app.schedule_save();
        });
    }
    let down = icon_button("go-down-symbolic", "Bajar bloque");
    {
        let app = app.clone();
        let id = id.clone();
        down.connect_clicked(move |_| {
            app.state.borrow_mut().move_block(&id, 1);
            app.refresh_editor();
            app.schedule_save();
        });
    }
    let anchor = icon_button(
        if block.placement.is_free() {
            "view-restore-symbolic"
        } else {
            "pin-symbolic"
        },
        if block.placement.is_free() {
            "Volver al flujo"
        } else {
            "Anclar bloque"
        },
    );
    {
        let app = app.clone();
        let id = id.clone();
        let free = !block.placement.is_free();
        anchor.connect_clicked(move |_| {
            app.state.borrow_mut().anchor_block(&id, free);
            app.refresh_editor();
            app.schedule_save();
        });
    }
    let duplicate = icon_button("edit-copy-symbolic", "Duplicar bloque");
    {
        let app = app.clone();
        let id = id.clone();
        duplicate.connect_clicked(move |_| {
            app.state.borrow_mut().duplicate_block(&id);
            app.refresh_editor();
            app.schedule_save();
        });
    }
    let remove = icon_button("user-trash-symbolic", "Eliminar bloque");
    {
        let app = app.clone();
        let id = id.clone();
        remove.connect_clicked(move |_| {
            app.state.borrow_mut().remove_block(&id);
            app.refresh_editor();
            app.schedule_save();
        });
    }
    row.append(&up);
    row.append(&down);
    row.append(&anchor);
    row.append(&duplicate);
    row.append(&remove);
    row.upcast()
}

pub fn kind_label(kind: &str) -> String {
    match kind {
        "text" => "Texto",
        "table" => "Tabla",
        "image" => "Imagen",
        "file" => "Archivo",
        "audio" => "Audio",
        "video" => "Vídeo",
        "web" => "Web",
        "drawing" => "Dibujo",
        "plugin" => "Plugin",
        other => other,
    }
    .to_string()
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.add_css_class("flat");
    button.set_tooltip_text(Some(tooltip));
    button.update_property(&[gtk::accessible::Property::Label(tooltip)]);
    button
}

fn render_table(app: &Rc<App>, block: &Block, table: &TableContent, editable: bool) -> gtk::Widget {
    let grid = gtk::Grid::new();
    grid.set_row_spacing(1);
    grid.set_column_spacing(1);
    grid.add_css_class("folio-table");
    for row in 0..table.rows {
        for col in 0..table.cols {
            let text = table
                .cell(row, col)
                .map(|cell| cell.text())
                .unwrap_or_default();
            if editable {
                let entry = gtk::Entry::new();
                entry.set_text(&text);
                entry.set_hexpand(true);
                if row == 0 && table.header {
                    entry.add_css_class("table-header");
                }
                let app = app.clone();
                let block_id = block.id.clone();
                entry.connect_changed(move |entry| {
                    let text = entry.text().to_string();
                    let mut state = app.state.borrow_mut();
                    if let Some(note) = state.current_note.as_mut()
                        && let Some(block) = note.block_mut(&block_id)
                        && let BlockContent::Table(table) = &mut block.content
                    {
                        table.set_cell(row, col, &text);
                        state.dirty = true;
                    }
                    drop(state);
                    app.schedule_save();
                });
                grid.attach(&entry, col as i32, row as i32, 1, 1);
            } else {
                let label = gtk::Label::new(Some(&text));
                label.set_xalign(0.0);
                label.set_widget_name(row.to_string().as_str());
                label.add_css_class(if row == 0 && table.header {
                    "table-header"
                } else {
                    "table-cell"
                });
                grid.attach(&label, col as i32, row as i32, 1, 1);
            }
        }
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_child(Some(&grid));
    scroller.set_hexpand(true);
    scroller.set_min_content_height((table.rows as i32 * 36).min(400));
    scroller.upcast()
}

fn render_image(app: &Rc<App>, image: &folio_core::model::ImageContent) -> gtk::Widget {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
    match app.state.borrow().store.asset_display_path(&image.asset) {
        Ok(Some(path)) => {
            let picture = gtk::Picture::for_filename(path);
            picture.set_can_shrink(true);
            picture.set_content_fit(gtk::ContentFit::Contain);
            picture.set_size_request(320, 180);
            box_.append(&picture);
        }
        _ => {
            let placeholder = gtk::Label::new(Some("Imagen no encontrada"));
            placeholder.add_css_class("missing");
            box_.append(&placeholder);
        }
    }
    if !image.caption.is_empty() {
        let caption = gtk::Label::new(Some(&image.caption));
        caption.add_css_class("caption");
        box_.append(&caption);
    }
    box_.upcast()
}

fn render_file(app: &Rc<App>, file: &folio_core::model::FileContent) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.add_css_class("file-card");
    let label = gtk::Label::new(Some(&format!(
        "📎 {} · {}",
        file.name,
        human_size(file.size)
    )));
    label.set_hexpand(true);
    label.set_xalign(0.0);
    row.append(&label);
    let open = gtk::Button::with_label("Abrir");
    let app = app.clone();
    let asset = file.asset.clone();
    let name = file.name.clone();
    open.connect_clicked(move |_| {
        let path = app
            .state
            .borrow()
            .store
            .asset_display_path(&asset)
            .ok()
            .flatten();
        if let Some(path) = path {
            let dialog = gtk::AlertDialog::builder()
                .message(format!("¿Abrir «{name}» con la aplicación del sistema?"))
                .buttons(["Cancelar", "Abrir"])
                .cancel_button(0)
                .default_button(0)
                .build();
            let path = path.clone();
            dialog.choose(
                Some(&app.window),
                None::<&gio::Cancellable>,
                move |result| {
                    if result == Ok(1) {
                        let uri = format!("file://{}", path.display());
                        let _ = gio::AppInfo::launch_default_for_uri(
                            &uri,
                            None::<&gio::AppLaunchContext>,
                        );
                    }
                },
            );
        }
    });
    row.append(&open);
    row.upcast()
}

fn human_size(bytes: u64) -> String {
    if bytes > 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    } else if bytes > 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn render_media(app: &Rc<App>, asset: &str, title: &str, video: bool) -> gtk::Widget {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let label = gtk::Label::new(Some(&format!(
        "{} {}",
        if video { "🎬" } else { "🔊" },
        title
    )));
    label.set_xalign(0.0);
    box_.append(&label);
    match app.state.borrow().store.asset_display_path(asset) {
        Ok(Some(path)) => {
            let media = gtk::Video::for_filename(Some(&path));
            media.set_autoplay(false);
            media.set_size_request(-1, if video { 220 } else { 64 });
            box_.append(&media);
        }
        _ => {
            let missing = gtk::Label::new(Some("Medio no encontrado"));
            missing.add_css_class("missing");
            box_.append(&missing);
        }
    }
    box_.upcast()
}

fn render_web(app: &Rc<App>, block: &Block, web: &folio_core::model::WebContent) -> gtk::Widget {
    if web.mode == "live" {
        let enabled = app.state.borrow().config.bool("web.live_enabled");
        if enabled {
            #[cfg(feature = "webkit")]
            {
                return crate::live_web::render(app, block, web);
            }
        }
        let hint = if enabled {
            "La vista viva no está disponible en esta compilación; compila con --features webkit."
        } else {
            "La vista viva está desactivada (web.live_enabled = false). Se muestra la tarjeta segura."
        };
        let card = web_card(app, block, web);
        let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let label = gtk::Label::new(Some(hint));
        label.set_wrap(true);
        label.set_xalign(0.0);
        label.add_css_class("caption");
        wrapper.append(&label);
        wrapper.append(&card);
        return wrapper.upcast();
    }
    web_card(app, block, web)
}

fn web_card(app: &Rc<App>, block: &Block, web: &folio_core::model::WebContent) -> gtk::Widget {
    let _ = block;
    let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
    card.add_css_class("web-card");
    let title = gtk::Label::new(Some(if web.title.is_empty() {
        &web.url
    } else {
        &web.title
    }));
    title.set_xalign(0.0);
    title.add_css_class("web-title");
    card.append(&title);
    if !web.description.is_empty() {
        let description = gtk::Label::new(Some(&web.description));
        description.set_wrap(true);
        description.set_xalign(0.0);
        card.append(&description);
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let url = gtk::Label::new(Some(&web.url));
    url.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    url.set_hexpand(true);
    url.set_xalign(0.0);
    row.append(&url);
    let open = gtk::Button::with_label("Abrir enlace");
    open.set_sensitive(folio_core::validate::validate_url(&web.url).is_ok());
    let app_open = app.clone();
    let web_url = web.url.clone();
    open.connect_clicked(move |_| {
        let dialog = gtk::AlertDialog::builder()
            .message(format!("¿Abrir {} en el navegador?", web_url))
            .buttons(["Cancelar", "Abrir"])
            .cancel_button(0)
            .default_button(0)
            .build();
        let web_url = web_url.clone();
        dialog.choose(
            Some(&app_open.window),
            None::<&gio::Cancellable>,
            move |result| {
                if result == Ok(1) {
                    let _ = gio::AppInfo::launch_default_for_uri(
                        &web_url,
                        None::<&gio::AppLaunchContext>,
                    );
                }
            },
        );
    });
    row.append(&open);
    #[cfg(feature = "webkit")]
    {
        if app.state.borrow().config.bool("web.live_enabled") {
            let live = gtk::Button::with_label("Usar vista viva");
            live.set_tooltip_text(Some(
                "Perfil efímero: sin cookies, sin descargas, permisos denegados",
            ));
            let app = app.clone();
            let block_id = block.id.clone();
            live.connect_clicked(move |_| {
                {
                    let mut state = app.state.borrow_mut();
                    if let Some(note) = state.current_note.as_mut()
                        && let Some(block) = note.block_mut(&block_id)
                        && let BlockContent::Web(web) = &mut block.content
                    {
                        web.mode = "live".into();
                        state.dirty = true;
                    }
                }
                app.refresh_editor();
                app.schedule_save();
            });
            row.append(&live);
        }
    }
    card.append(&row);
    if web.fetched_at.is_none() {
        let hint = gtk::Label::new(Some(
            "Sin metadatos: se muestra el enlace (funciona sin red)",
        ));
        hint.add_css_class("caption");
        hint.set_xalign(0.0);
        card.append(&hint);
    }
    card.upcast()
}

fn render_drawing(
    app: &Rc<App>,
    block: &Block,
    drawing: &DrawingContent,
    editable: bool,
) -> gtk::Widget {
    let area = gtk::DrawingArea::new();
    area.set_content_width(600);
    area.set_content_height(300);
    area.add_css_class("drawing-area");
    let strokes = drawing.strokes.clone();
    let width = drawing.width.max(1.0);
    let height = drawing.height.max(1.0);
    area.set_draw_func(move |_, cr, w, h| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.paint();
        let scale_x = w as f64 / width;
        let scale_y = h as f64 / height;
        for stroke in &strokes {
            if stroke.points.len() < 2 {
                continue;
            }
            let color = parse_color(&stroke.color);
            cr.set_source_rgba(color.0, color.1, color.2, 1.0);
            cr.set_line_width(stroke.width.max(0.5));
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            cr.set_line_join(gtk::cairo::LineJoin::Round);
            for (index, point) in stroke.points.iter().enumerate() {
                if point.len() < 2 {
                    continue;
                }
                let x = point[0] * scale_x;
                let y = point[1] * scale_y;
                if index == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            let _ = cr.stroke();
        }
    });
    if editable {
        let gesture = gtk::GestureDrag::new();
        gesture.set_button(1);
        let active: Rc<std::cell::RefCell<Option<Stroke>>> = Rc::new(std::cell::RefCell::new(None));
        let app_begin = app.clone();
        let block_id = block.id.clone();
        let active_begin = active.clone();
        gesture.connect_drag_begin(move |_, x, y| {
            let _ = &app_begin;
            let _ = &block_id;
            *active_begin.borrow_mut() = Some(Stroke {
                tool: "pen".into(),
                color: "#1a1a1a".into(),
                width: 2.5,
                points: vec![vec![x, y, 0.5, 0.0]],
                extra: Default::default(),
            });
        });
        let area_update = area.clone();
        let active_update = active.clone();
        gesture.connect_drag_update(move |_, dx, dy| {
            if let Some(stroke) = active_update.borrow_mut().as_mut() {
                let last = stroke.points.last().cloned().unwrap_or_default();
                let x = last.first().copied().unwrap_or(0.0) + dx;
                let y = last.get(1).copied().unwrap_or(0.0) + dy;
                stroke.points.push(vec![x, y, 0.5, 0.0]);
                let _ = &area_update;
                area_update.queue_draw();
            }
        });
        let app_end = app.clone();
        let block_id = block.id.clone();
        let active_end = active.clone();
        gesture.connect_drag_end(move |_, _, _| {
            if let Some(mut stroke) = active_end.borrow_mut().take() {
                let mut state = app_end.state.borrow_mut();
                if let Some(note) = state.current_note.as_mut()
                    && let Some(block) = note.block_mut(&block_id)
                    && let BlockContent::Drawing(drawing) = &mut block.content
                {
                    stroke.points = stroke
                        .points
                        .iter()
                        .map(|point| {
                            vec![
                                point.first().copied().unwrap_or(0.0),
                                point.get(1).copied().unwrap_or(0.0),
                                point.get(2).copied().unwrap_or(0.5),
                                point.get(3).copied().unwrap_or(0.0),
                            ]
                        })
                        .collect();
                    drawing.strokes.push(stroke);
                    state.dirty = true;
                }
                drop(state);
                app_end.schedule_save();
                app_end.refresh_editor();
            }
        });
        area.add_controller(gesture);
    }
    area.upcast()
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

fn render_plugin(app: &Rc<App>, block: &Block, plugin: &PluginContent) -> gtk::Widget {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 8);
    container.add_css_class("plugin-block");
    let response = {
        let state = app.state.borrow();
        state.plugin_run(
            &plugin.plugin,
            &folio_plugin_sdk::Request {
                command: "render".into(),
                block_state: plugin.state.clone(),
                payload: serde_json::Value::Null,
            },
        )
    };
    match response {
        Ok(response) => {
            container.append(&render_ui_node(app, block, &response.ui));
        }
        Err(folio_plugins::PluginError::NotFound(_)) => {
            let label = gtk::Label::new(Some(&format!(
                "Plugin «{}» no instalado. El contenido se conserva.",
                plugin.plugin
            )));
            label.set_wrap(true);
            container.append(&label);
        }
        Err(error) => {
            container.append(&gtk::Label::new(Some(&format!(
                "El plugin falló de forma aislada: {error}"
            ))));
        }
    }
    container.upcast()
}

pub fn render_ui_node(app: &Rc<App>, block: &Block, node: &UiNode) -> gtk::Widget {
    match node {
        UiNode::Column { children } => {
            let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
            for child in children {
                box_.append(&render_ui_node(app, block, child));
            }
            box_.upcast()
        }
        UiNode::Row { children } => {
            let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            for child in children {
                box_.append(&render_ui_node(app, block, child));
            }
            box_.upcast()
        }
        UiNode::Label { text, style } => {
            let label = gtk::Label::new(Some(text));
            label.set_wrap(true);
            label.set_xalign(0.0);
            match style {
                LabelStyle::Title => label.add_css_class("plugin-title"),
                LabelStyle::Caption => label.add_css_class("caption"),
                LabelStyle::Monospace => label.add_css_class("monospace"),
                LabelStyle::Body => {}
            }
            label.upcast()
        }
        UiNode::Heading { text } => {
            let label = gtk::Label::new(Some(text));
            label.set_xalign(0.0);
            label.add_css_class("plugin-title");
            label.upcast()
        }
        UiNode::Button {
            id,
            label,
            disabled,
        } => {
            let button = gtk::Button::with_label(label);
            button.set_sensitive(!disabled);
            let app = app.clone();
            let block_id = block.id.clone();
            let command = id.clone();
            button.connect_clicked(move |_| {
                let _ = app.state.borrow_mut().plugin_command(
                    &block_id,
                    &command,
                    serde_json::Value::Null,
                );
                app.refresh_editor();
                app.schedule_save();
            });
            button.upcast()
        }
        UiNode::Checkbox { id, label, checked } => {
            let check = gtk::CheckButton::with_label(label);
            check.set_active(*checked);
            let app = app.clone();
            let block_id = block.id.clone();
            let command = id.clone();
            check.connect_toggled(move |check| {
                let _ = app.state.borrow_mut().plugin_command(
                    &block_id,
                    &command,
                    serde_json::json!({"checked": check.is_active()}),
                );
                app.schedule_save();
            });
            check.upcast()
        }
        UiNode::Slider {
            id,
            min,
            max,
            value,
        } => {
            let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, *min, *max, 0.01);
            scale.set_value(*value);
            let app = app.clone();
            let block_id = block.id.clone();
            let command = id.clone();
            scale.connect_value_changed(move |scale| {
                let _ = app.state.borrow_mut().plugin_command(
                    &block_id,
                    &command,
                    serde_json::json!({"value": scale.value()}),
                );
            });
            scale.upcast()
        }
        UiNode::Progress {
            label: _,
            value,
            max,
        } => {
            let bar = gtk::ProgressBar::new();
            bar.set_fraction(if *max > 0.0 { value / max } else { 0.0 });
            bar.set_show_text(true);
            bar.upcast()
        }
        UiNode::Chart {
            kind,
            title,
            series,
        } => {
            let area = gtk::DrawingArea::new();
            area.set_content_height(180);
            let kind = *kind;
            let title = title.clone();
            let series = series.clone();
            area.set_draw_func(move |_, cr, w, h| {
                draw_chart(cr, w, h, kind, &title, &series);
            });
            area.upcast()
        }
        UiNode::Diagram { nodes, edges } => {
            let area = gtk::DrawingArea::new();
            area.set_content_height(220);
            let nodes = nodes.clone();
            let edges = edges.clone();
            area.set_draw_func(move |_, cr, w, h| {
                draw_diagram(cr, w, h, &nodes, &edges);
            });
            area.upcast()
        }
        UiNode::Table { headers, rows } => {
            let grid = gtk::Grid::new();
            grid.set_row_spacing(2);
            grid.set_column_spacing(8);
            for (col, header) in headers.iter().enumerate() {
                let label = gtk::Label::new(Some(header));
                label.set_xalign(0.0);
                label.add_css_class("table-header");
                grid.attach(&label, col as i32, 0, 1, 1);
            }
            for (row, cells) in rows.iter().enumerate() {
                for (col, cell) in cells.iter().enumerate() {
                    let label = gtk::Label::new(Some(cell));
                    label.set_xalign(0.0);
                    grid.attach(&label, col as i32, row as i32 + 1, 1, 1);
                }
            }
            grid.upcast()
        }
        UiNode::Separator => gtk::Separator::new(gtk::Orientation::Horizontal).upcast(),
        UiNode::Spacer { height } => {
            let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
            spacer.set_size_request(-1, (*height).max(0.0) as i32);
            spacer.upcast()
        }
    }
}

fn draw_chart(
    cr: &gtk::cairo::Context,
    w: i32,
    h: i32,
    kind: ChartKind,
    title: &str,
    series: &[folio_plugin_sdk::Series],
) {
    let w = w as f64;
    let h = h as f64;
    cr.set_source_rgb(0.98, 0.98, 0.98);
    let _ = cr.paint();
    cr.set_source_rgb(0.1, 0.1, 0.1);
    cr.select_font_face(
        "Sans",
        gtk::cairo::FontSlant::Normal,
        gtk::cairo::FontWeight::Bold,
    );
    cr.set_font_size(14.0);
    cr.move_to(12.0, 22.0);
    let _ = cr.show_text(title);
    let values: Vec<f64> = series.iter().flat_map(|s| s.values.clone()).collect();
    let max = values.iter().cloned().fold(0.0f64, f64::max).max(1.0);
    let plot_h = (h - 50.0).max(10.0);
    let count = values.len().max(1);
    let bar_w = ((w - 40.0) / count as f64 * 0.6).max(4.0);
    let palette = [
        (0.21, 0.52, 0.89),
        (0.20, 0.70, 0.55),
        (0.95, 0.55, 0.25),
        (0.75, 0.35, 0.75),
        (0.90, 0.75, 0.20),
    ];
    match kind {
        ChartKind::Line => {
            cr.set_line_width(2.0);
            cr.set_source_rgb(0.21, 0.52, 0.89);
            for (index, value) in values.iter().enumerate() {
                let x = 30.0 + index as f64 * (w - 50.0) / (count as f64).max(1.0);
                let y = h - 20.0 - (value / max) * plot_h;
                if index == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            let _ = cr.stroke();
        }
        ChartKind::Pie => {
            let total: f64 = values.iter().sum();
            let mut angle = -std::f64::consts::FRAC_PI_2;
            let cx = w / 2.0;
            let cy = h / 2.0 + 8.0;
            let radius = plot_h.min(w) * 0.35;
            for (index, value) in values.iter().enumerate() {
                let sweep = if total > 0.0 {
                    value / total * std::f64::consts::TAU
                } else {
                    0.0
                };
                let color = palette[index % palette.len()];
                cr.set_source_rgb(color.0, color.1, color.2);
                cr.move_to(cx, cy);
                cr.arc(cx, cy, radius, angle, angle + sweep);
                cr.close_path();
                let _ = cr.fill();
                angle += sweep;
            }
        }
        ChartKind::Bar => {
            for (index, value) in values.iter().enumerate() {
                let x = 30.0 + index as f64 * (w - 50.0) / (count as f64).max(1.0);
                let bar_h = (value / max) * plot_h;
                let color = palette[index % palette.len()];
                cr.set_source_rgb(color.0, color.1, color.2);
                cr.rectangle(x, h - 20.0 - bar_h, bar_w, bar_h);
                let _ = cr.fill();
            }
        }
    }
}

fn draw_diagram(
    cr: &gtk::cairo::Context,
    w: i32,
    h: i32,
    nodes: &[folio_plugin_sdk::DiagramNode],
    edges: &[folio_plugin_sdk::DiagramEdge],
) {
    let w = w as f64;
    cr.set_source_rgb(0.98, 0.98, 0.98);
    let _ = cr.paint();
    if nodes.is_empty() {
        return;
    }
    let columns = nodes.len().clamp(1, 4);
    let cell_w = w / columns as f64;
    let cell_h = (h as f64 - 20.0) / ((nodes.len() as f64 / columns as f64).ceil());
    let position = |id: &str| -> Option<(f64, f64)> {
        let index = nodes.iter().position(|node| node.id == id)?;
        let col = index % columns;
        let row = index / columns;
        Some((
            col as f64 * cell_w + cell_w / 2.0,
            row as f64 * cell_h + cell_h / 2.0,
        ))
    };
    cr.set_source_rgb(0.35, 0.35, 0.35);
    cr.set_line_width(1.2);
    for edge in edges {
        if let (Some((x1, y1)), Some((x2, y2))) = (position(&edge.from), position(&edge.to)) {
            cr.move_to(x1, y1);
            cr.line_to(x2, y2);
            let _ = cr.stroke();
        }
    }
    cr.select_font_face(
        "Sans",
        gtk::cairo::FontSlant::Normal,
        gtk::cairo::FontWeight::Normal,
    );
    cr.set_font_size(11.0);
    for (index, node) in nodes.iter().enumerate() {
        let col = index % columns;
        let row = index / columns;
        let x = col as f64 * cell_w + 8.0;
        let y = row as f64 * cell_h + 8.0;
        let box_w = cell_w - 16.0;
        let box_h = 36.0;
        cr.set_source_rgb(0.86, 0.92, 0.99);
        rounded_rect(cr, x, y, box_w, box_h, 6.0);
        let _ = cr.fill();
        cr.set_source_rgb(0.1, 0.1, 0.1);
        let text_width = node.label.chars().count() as f64 * 6.0;
        cr.move_to(x + (box_w - text_width) / 2.0, y + box_h / 2.0 + 4.0);
        let _ = cr.show_text(&node.label);
    }
}

fn rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        1.5 * std::f64::consts::PI,
    );
    cr.close_path();
}

pub fn canvas_block_widget(
    app: &Rc<App>,
    block: &Block,
    rect: folio_layout::Rect,
    scale: f64,
) -> gtk::Widget {
    let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 0);
    wrapper.add_css_class("canvas-item");
    wrapper.set_size_request(
        (rect.w * scale).max(80.0) as i32,
        (rect.h * scale).max(40.0) as i32,
    );
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    header.add_css_class("block-header");
    let label = gtk::Label::new(Some(&kind_label(&block.kind)));
    label.set_hexpand(true);
    label.set_xalign(0.0);
    header.append(&label);
    let anchor = icon_button("view-restore-symbolic", "Volver al flujo");
    {
        let app = app.clone();
        let id = block.id.clone();
        anchor.connect_clicked(move |_| {
            app.state.borrow_mut().anchor_block(&id, false);
            app.refresh_editor();
            app.schedule_save();
        });
    }
    header.append(&anchor);
    wrapper.append(&header);
    let content = block_body(app, block, true);
    wrapper.append(&content);
    // Arrastrar ancla el bloque en coordenadas del mundo.
    let gesture = gtk::GestureDrag::new();
    let start_rect = rect;
    let app_drag = app.clone();
    let block_id = block.id.clone();
    let start_pos = Rc::new(std::cell::RefCell::new((0.0, 0.0)));
    let start_pos_begin = start_pos.clone();
    gesture.connect_drag_begin(move |_, x, y| {
        *start_pos_begin.borrow_mut() = (x, y);
    });
    let start_pos_end = start_pos.clone();
    gesture.connect_drag_end(move |_, dx, dy| {
        let (_, _) = *start_pos_end.borrow();
        let x = start_rect.x + dx / scale;
        let y = start_rect.y + dy / scale;
        app_drag.state.borrow_mut().set_placement(
            &block_id,
            Placement::free(x.max(0.0), y.max(0.0), start_rect.w, start_rect.h),
        );
        app_drag.refresh_editor();
        app_drag.schedule_save();
    });
    header.add_controller(gesture);
    wrapper.upcast()
}

pub fn estimate(block: &Block, width: f64) -> f64 {
    estimate_height(block, width)
}

pub fn _unused_gdk() -> Option<gdk::Texture> {
    None
}
