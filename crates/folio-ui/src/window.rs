use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::rc::Weak;
use std::time::Duration;

use folio_core::{Block, FolioError, model::BlockContent};
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::blocks;
use crate::dialogs;
use crate::state::{AppState, ViewMode};

pub struct App {
    pub state: RefCell<AppState>,
    pub window: adw::ApplicationWindow,
    pub application: adw::Application,
    pub sidebar: gtk::ListBox,
    pub notes: gtk::ListBox,
    pub notes_title: gtk::Label,
    pub title_entry: gtk::Entry,
    pub editor: gtk::Box,
    pub editor_scroll: gtk::ScrolledWindow,
    pub status: gtk::Label,
    pub search_entry: gtk::SearchEntry,
    pub overlay: adw::ToastOverlay,
    pub suppress: Cell<bool>,
    pub suppress_title: Cell<bool>,
    pub focused: RefCell<Option<gtk::TextView>>,
    pub autosave: Cell<Option<glib::SourceId>>,
    pub mode_label: gtk::Label,
    self_ref: RefCell<Weak<App>>,
    config_monitors: RefCell<Vec<gio::FileMonitor>>,
}

impl App {
    pub fn new(state: AppState, application: &adw::Application) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("Folio")
            .default_width(1240)
            .default_height(820)
            .build();

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let toolbar = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        let title = adw::WindowTitle::new("Folio", "Notas y libros diarios");
        header.set_title_widget(Some(&title));

        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some("Menú principal"));
        let menu = build_menu();
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);

        let new_note = gtk::Button::from_icon_name("document-new-symbolic");
        new_note.set_tooltip_text(Some("Nueva nota (Ctrl+N)"));
        header.pack_start(&new_note);

        let new_book = gtk::Button::from_icon_name("folder-new-symbolic");
        new_book.set_tooltip_text(Some("Nuevo libro (Ctrl+Shift+N)"));
        header.pack_start(&new_book);

        toolbar.add_top_bar(&header);

        // Sidebar
        let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        sidebar_box.set_size_request(250, -1);
        sidebar_box.add_css_class("sidebar");
        let sidebar_header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        sidebar_header.set_margin_top(8);
        sidebar_header.set_margin_start(12);
        sidebar_header.set_margin_end(8);
        let library = gtk::Label::new(Some("Biblioteca"));
        library.set_xalign(0.0);
        library.set_hexpand(true);
        library.add_css_class("pane-title");
        sidebar_header.append(&library);
        let today = gtk::Button::from_icon_name("x-office-calendar-symbolic");
        today.set_tooltip_text(Some("Ir a hoy (Ctrl+Shift+H)"));
        sidebar_header.append(&today);
        sidebar_box.append(&sidebar_header);
        let sidebar_scroll = gtk::ScrolledWindow::new();
        sidebar_scroll.set_vexpand(true);
        sidebar_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        let sidebar = gtk::ListBox::new();
        sidebar.set_selection_mode(gtk::SelectionMode::Single);
        sidebar.add_css_class("navigation-sidebar");
        sidebar_scroll.set_child(Some(&sidebar));
        sidebar_box.append(&sidebar_scroll);

        // Notes pane
        let notes_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        notes_box.set_size_request(300, -1);
        let notes_header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        notes_header.set_margin_top(8);
        notes_header.set_margin_start(12);
        notes_header.set_margin_end(8);
        let notes_title = gtk::Label::new(Some("Notas"));
        notes_title.set_xalign(0.0);
        notes_title.set_hexpand(true);
        notes_title.add_css_class("pane-title");
        notes_header.append(&notes_title);
        let add_note = gtk::Button::from_icon_name("list-add-symbolic");
        add_note.set_tooltip_text(Some("Nueva nota"));
        notes_header.append(&add_note);
        notes_box.append(&notes_header);
        let search_entry = gtk::SearchEntry::new();
        search_entry.set_placeholder_text(Some("Buscar en toda la bóveda"));
        search_entry.set_margin_start(8);
        search_entry.set_margin_end(8);
        notes_box.append(&search_entry);
        let notes_scroll = gtk::ScrolledWindow::new();
        notes_scroll.set_vexpand(true);
        notes_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        let notes = gtk::ListBox::new();
        notes.set_selection_mode(gtk::SelectionMode::Single);
        notes.add_css_class("notes-list");
        notes_scroll.set_child(Some(&notes));
        notes_box.append(&notes_scroll);

        // Editor pane
        let editor_pane = gtk::Box::new(gtk::Orientation::Vertical, 0);
        editor_pane.set_hexpand(true);
        let title_entry = gtk::Entry::new();
        title_entry.set_placeholder_text(Some("Título de la nota"));
        title_entry.add_css_class("note-title-entry");
        title_entry.set_margin_top(14);
        title_entry.set_margin_start(24);
        title_entry.set_margin_end(24);
        editor_pane.append(&title_entry);

        let format_bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        format_bar.set_margin_start(18);
        format_bar.set_margin_end(18);
        format_bar.set_margin_top(4);
        format_bar.add_css_class("format-bar");
        for (icon, tag, tooltip) in [
            ("format-text-bold-symbolic", "bold", "Negrita"),
            ("format-text-italic-symbolic", "italic", "Cursiva"),
            ("format-text-underline-symbolic", "underline", "Subrayado"),
            (
                "format-text-strikethrough-symbolic",
                "strikethrough",
                "Tachado",
            ),
            ("format-text-code-symbolic", "code", "Código"),
        ] {
            let button = gtk::Button::from_icon_name(icon);
            button.add_css_class("flat");
            button.set_tooltip_text(Some(tooltip));
            // La acción se conecta después de crear App.
            button.set_widget_name(tag);
            button.set_action_name(Some(&format!("win.format-{tag}")));
            format_bar.append(&button);
        }
        for (label, tag) in [
            ("H1", "h1"),
            ("H2", "h2"),
            ("H3", "h3"),
            ("❝", "quote"),
            ("•", "list"),
            ("☑", "checklist"),
        ] {
            let button = gtk::Button::with_label(label);
            button.add_css_class("flat");
            button.set_tooltip_text(Some(tag));
            button.set_action_name(Some(&format!("win.style-{tag}")));
            format_bar.append(&button);
        }
        let link = gtk::Button::from_icon_name("insert-link-symbolic");
        link.add_css_class("flat");
        link.set_tooltip_text(Some("Enlace"));
        link.set_action_name(Some("win.format-link"));
        format_bar.append(&link);
        let add_block = gtk::MenuButton::new();
        add_block.set_icon_name("list-add-symbolic");
        add_block.set_tooltip_text(Some("Insertar bloque"));
        add_block.set_menu_model(Some(&build_block_menu()));
        add_block.add_css_class("flat");
        format_bar.append(&add_block);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        format_bar.append(&spacer);
        let preview = gtk::Button::from_icon_name("view-reveal-symbolic");
        preview.add_css_class("flat");
        preview.set_tooltip_text(Some("Previsualización (Ctrl+Shift+E)"));
        preview.set_action_name(Some("win.preview"));
        format_bar.append(&preview);
        let canvas = gtk::Button::from_icon_name("view-fullscreen-symbolic");
        canvas.add_css_class("flat");
        canvas.set_tooltip_text(Some("Lienzo"));
        canvas.set_action_name(Some("win.canvas"));
        format_bar.append(&canvas);
        editor_pane.append(&format_bar);

        let editor_scroll = gtk::ScrolledWindow::new();
        editor_scroll.set_vexpand(true);
        editor_scroll.set_hexpand(true);
        let editor = gtk::Box::new(gtk::Orientation::Vertical, 8);
        editor.set_margin_start(32);
        editor.set_margin_end(32);
        editor.set_margin_top(12);
        editor.set_margin_bottom(48);
        editor.set_hexpand(true);
        editor_scroll.set_child(Some(&editor));
        editor_pane.append(&editor_scroll);

        let status = gtk::Label::new(Some("Listo"));
        status.set_xalign(0.0);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        status.add_css_class("status-bar");
        let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        status_box.add_css_class("status-bar-box");
        status_box.append(&status);
        let mode_label = gtk::Label::new(Some("Documento"));
        mode_label.add_css_class("caption");
        status_box.append(&mode_label);
        editor_pane.append(&status_box);

        let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        paned.set_start_child(Some(&sidebar_box));
        paned.set_shrink_start_child(false);
        let right = gtk::Paned::new(gtk::Orientation::Horizontal);
        right.set_start_child(Some(&notes_box));
        right.set_shrink_start_child(false);
        right.set_end_child(Some(&editor_pane));
        right.set_resize_end_child(true);
        paned.set_end_child(Some(&right));
        paned.set_resize_end_child(true);
        paned.set_position(250);
        right.set_position(300);

        let overlay = adw::ToastOverlay::new();
        overlay.set_child(Some(&paned));
        root.append(&overlay);
        toolbar.set_content(Some(&root));
        window.set_content(Some(&toolbar));

        let app = Rc::new(Self {
            state: RefCell::new(state),
            window: window.clone(),
            application: application.clone(),
            sidebar,
            notes,
            notes_title,
            title_entry,
            editor,
            editor_scroll,
            status,
            search_entry,
            overlay,
            suppress: Cell::new(false),
            suppress_title: Cell::new(false),
            focused: RefCell::new(None),
            autosave: Cell::new(None),
            mode_label,
            self_ref: RefCell::new(Weak::new()),
            config_monitors: RefCell::new(Vec::new()),
        });
        *app.self_ref.borrow_mut() = Rc::downgrade(&app);

        setup_actions(&app);
        connect_widgets(&app, &new_note, &new_book, &add_note, &today);
        app.install_config_watch();
        app.install_day_watch();
        app.refresh_all();
        app.window.present();
        app
    }

    pub fn state(&self) -> std::cell::Ref<'_, AppState> {
        self.state.borrow()
    }

    pub fn set_status(&self, message: &str) {
        self.status.set_text(message);
    }

    pub fn toast(&self, message: &str) {
        self.overlay.add_toast(adw::Toast::new(message));
    }

    pub fn report(&self, error: &FolioError) {
        let message = format!("{} · {}", error, error.action());
        self.toast(&message);
        self.set_status(&message);
        tracing::warn!("{message}");
    }

    pub fn report_str(&self, error: impl std::fmt::Display) {
        let message = error.to_string();
        self.toast(&message);
        self.set_status(&message);
        tracing::warn!("{message}");
    }

    pub fn refresh_all(&self) {
        if let Err(error) = self.refresh_sidebar_inner() {
            self.report(&error);
        }
        if let Err(error) = self.refresh_notes_inner() {
            self.report(&error);
        }
        self.refresh_editor();
    }

    fn refresh_sidebar_inner(&self) -> Result<(), FolioError> {
        while let Some(child) = self.sidebar.first_child() {
            self.sidebar.remove(&child);
        }
        let state = self.state.borrow();
        let books = state.books()?;
        let current = state.current_book.clone();
        let mode = state.mode;
        let note_counts: Vec<usize> = books
            .iter()
            .map(|book| {
                state
                    .store
                    .notes(book)
                    .map(|notes| notes.len())
                    .unwrap_or(0)
            })
            .collect();
        drop(state);
        for (book, count) in books.iter().zip(note_counts) {
            let row = gtk::ListBoxRow::new();
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            content.set_margin_top(6);
            content.set_margin_bottom(6);
            content.set_margin_start(8);
            content.set_margin_end(8);
            let icon = gtk::Image::from_icon_name(if book.meta.kind == "daily" {
                "x-office-calendar-symbolic"
            } else {
                "x-office-document-symbolic"
            });
            content.append(&icon);
            let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let title = gtk::Label::new(Some(&book.meta.title));
            title.set_xalign(0.0);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            if book.meta.archived {
                title.add_css_class("dim-label");
            }
            let subtitle = gtk::Label::new(Some(&format!(
                "{} · {} nota{}",
                book.meta
                    .daily_date
                    .clone()
                    .unwrap_or_else(|| "manual".into()),
                count,
                if count == 1 { "" } else { "s" }
            )));
            subtitle.set_xalign(0.0);
            subtitle.add_css_class("caption");
            labels.append(&title);
            labels.append(&subtitle);
            labels.set_hexpand(true);
            content.append(&labels);
            if book.meta.pinned {
                content.append(&gtk::Image::from_icon_name("starred-symbolic"));
            }
            row.set_child(Some(&content));
            row.set_tooltip_text(Some(&book.folder));
            let id = book.meta.id.clone();
            let app = self.clone_ref();
            row.connect_activate(move |_| {
                if let Err(error) = app.state.borrow_mut().open_book(&id) {
                    app.report(&error);
                }
                app.refresh_notes_inner_public();
                app.refresh_editor();
                app.refresh_sidebar_inner_public();
            });
            // DnD: soltar una nota en un libro la mueve ahí.
            let drop = gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
            // Spring-load: 650 ms sobre un libro lo abre sin soltar la nota.
            let hover = Rc::new(Cell::new(false));
            let app_enter = self.clone_ref();
            let target_enter = book.meta.id.clone();
            let hover_enter = hover.clone();
            drop.connect_enter(move |_, _, _| {
                hover_enter.set(true);
                let app = app_enter.clone();
                let target = target_enter.clone();
                let hover = hover_enter.clone();
                glib::timeout_add_local_once(Duration::from_millis(650), move || {
                    if !hover.get() {
                        return;
                    }
                    if app.state.borrow_mut().open_book(&target).is_ok() {
                        app.refresh_notes_inner_public();
                        app.refresh_editor();
                        app.refresh_sidebar_inner_public();
                    }
                });
                gtk::gdk::DragAction::MOVE
            });
            let hover_leave = hover.clone();
            drop.connect_leave(move |_| hover_leave.set(false));
            let app_drop = self.clone_ref();
            let target = book.meta.id.clone();
            drop.connect_drop(move |_, value, _, _| {
                let Ok(note_id) = value.get::<String>() else {
                    return false;
                };
                if let Err(error) = app_drop
                    .state
                    .borrow_mut()
                    .move_note(&note_id, &target, None)
                {
                    app_drop.report(&error);
                    return false;
                }
                app_drop.refresh_all();
                app_drop.toast("Nota movida");
                true
            });
            row.add_controller(drop);
            if current.as_deref() == Some(book.meta.id.as_str()) {
                self.sidebar.select_row(Some(&row));
            }
            self.sidebar.append(&row);
        }
        let _ = mode;
        Ok(())
    }

    pub fn refresh_sidebar_inner_public(&self) {
        if let Err(error) = self.refresh_sidebar_inner() {
            self.report(&error);
        }
    }

    fn refresh_notes_inner(&self) -> Result<(), FolioError> {
        while let Some(child) = self.notes.first_child() {
            self.notes.remove(&child);
        }
        let state = self.state.borrow();
        let notes = state.notes()?;
        let current_note = state.current_note.as_ref().map(|note| note.id.clone());
        let book_title = state
            .current_book()
            .map(|book| book.meta.title)
            .unwrap_or_else(|| "Sin libro".into());
        let searching = !state.search.is_empty();
        drop(state);
        self.notes_title.set_text(&format!(
            "{}{}",
            if searching { "Resultados · " } else { "" },
            book_title
        ));
        for summary in notes {
            let row = gtk::ListBoxRow::new();
            let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
            content.set_margin_top(6);
            content.set_margin_bottom(6);
            content.set_margin_start(10);
            content.set_margin_end(10);
            let title = gtk::Label::new(Some(&summary.title));
            title.set_xalign(0.0);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let meta = gtk::Label::new(Some(&format!(
                "{} · {} palabras{}",
                summary.updated_at.get(0..10).unwrap_or(&summary.updated_at),
                summary.word_count,
                if summary.has_media { " · medios" } else { "" }
            )));
            meta.set_xalign(0.0);
            meta.add_css_class("caption");
            content.append(&title);
            content.append(&meta);
            row.set_child(Some(&content));
            row.set_tooltip_text(Some(&summary.id));
            let id = summary.id.clone();
            let app = self.clone_ref();
            row.connect_activate(move |_| {
                if let Err(error) = app.state.borrow_mut().open_note(&id) {
                    app.report(&error);
                }
                app.refresh_editor();
                app.refresh_notes_inner_public();
            });
            let drag = gtk::DragSource::new();
            drag.set_actions(gtk::gdk::DragAction::MOVE);
            let drag_id = summary.id.clone();
            drag.connect_prepare(move |_, _, _| {
                Some(gtk::gdk::ContentProvider::for_value(&drag_id.to_value()))
            });
            row.add_controller(drag);
            // Soltar sobre otra nota: insertar antes de ella.
            let drop = gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
            let app_drop = self.clone_ref();
            let target = summary.id.clone();
            drop.connect_drop(move |_, value, _, _| {
                let Ok(note_id) = value.get::<String>() else {
                    return false;
                };
                if note_id == target {
                    return false;
                }
                let book = app_drop.state.borrow().current_book.clone();
                let Some(book) = book else {
                    return false;
                };
                if let Err(error) =
                    app_drop
                        .state
                        .borrow_mut()
                        .move_note(&note_id, &book, Some(&target))
                {
                    app_drop.report(&error);
                    return false;
                }
                app_drop.refresh_all();
                true
            });
            row.add_controller(drop);
            if current_note.as_deref() == Some(summary.id.as_str()) {
                self.notes.select_row(Some(&row));
            }
            self.notes.append(&row);
        }
        Ok(())
    }

    pub fn refresh_notes_inner_public(&self) {
        if let Err(error) = self.refresh_notes_inner() {
            self.report(&error);
        }
    }

    pub fn refresh_editor(&self) {
        while let Some(child) = self.editor.first_child() {
            self.editor.remove(&child);
        }
        let state = self.state.borrow();
        let Some(note) = state.current_note.clone() else {
            drop(state);
            self.suppress_title.set(true);
            self.title_entry.set_text("");
            self.suppress_title.set(false);
            let empty = gtk::Box::new(gtk::Orientation::Vertical, 12);
            empty.set_valign(gtk::Align::Center);
            empty.set_halign(gtk::Align::Center);
            let label = gtk::Label::new(Some("Selecciona una nota o crea una nueva"));
            label.add_css_class("empty-title");
            empty.append(&label);
            let button = gtk::Button::with_label("Nueva nota");
            button.set_action_name(Some("win.new-note"));
            empty.append(&button);
            self.editor.append(&empty);
            self.mode_label.set_text("Sin nota");
            self.set_status("Listo");
            return;
        };
        let mode = state.mode;
        let scale = state.canvas_scale;
        let word_count = note.word_count();
        drop(state);
        self.suppress_title.set(true);
        self.title_entry.set_text(&note.title);
        self.suppress_title.set(false);
        self.mode_label.set_text(match mode {
            ViewMode::Document => "Documento",
            ViewMode::Canvas => "Lienzo",
            ViewMode::Preview => "Previsualización",
        });
        self.set_status(&format!("{word_count} palabras"));

        match mode {
            ViewMode::Preview => {
                let app = self.clone_ref();
                let banner = gtk::Label::new(Some(
                    "Previsualización de solo lectura · Ctrl+Shift+E para volver",
                ));
                banner.add_css_class("preview-banner");
                banner.set_xalign(0.0);
                self.editor.append(&banner);
                for block in &note.blocks {
                    let widget = blocks::block_widget(&app, block, false);
                    self.editor.append(&widget);
                }
            }
            ViewMode::Canvas => {
                let app = self.clone_ref();
                self.build_canvas(&app, &note, scale);
            }
            ViewMode::Document => {
                let app = self.clone_ref();
                for block in &note.blocks {
                    let widget = blocks::block_widget(&app, block, true);
                    self.editor.append(&widget);
                }
                let add = gtk::Button::with_label("+ Añadir bloque");
                add.add_css_class("flat");
                add.set_action_name(Some("win.add-text-block"));
                self.editor.append(&add);
            }
        }
    }

    fn build_canvas(&self, app: &Rc<App>, note: &folio_core::Note, scale: f64) {
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        controls.add_css_class("canvas-controls");
        let label = gtk::Label::new(Some("Zoom"));
        controls.append(&label);
        let zoom = gtk::Scale::with_range(gtk::Orientation::Horizontal, 25.0, 400.0, 5.0);
        zoom.set_value(scale * 100.0);
        zoom.set_hexpand(true);
        let app_zoom = self.clone_ref();
        zoom.connect_value_changed(move |zoom| {
            let value = zoom.value() / 100.0;
            app_zoom.state.borrow_mut().canvas_scale = value;
            app_zoom.refresh_editor_debounced();
        });
        controls.append(&zoom);
        let fit_doc = gtk::Button::with_label("Documento");
        fit_doc.set_action_name(Some("win.fit-document"));
        controls.append(&fit_doc);
        let fit_all = gtk::Button::with_label("Todo");
        fit_all.set_action_name(Some("win.fit-all"));
        controls.append(&fit_all);
        let recompose = gtk::Button::with_label("Recomponer");
        recompose.set_action_name(Some("win.recompose"));
        controls.append(&recompose);
        self.editor.append(&controls);

        let layout = folio_layout::layout_note(note, blocks::estimate);
        let fixed = gtk::Fixed::new();
        let total_w = (layout.bbox.right().max(layout.flow_width) * scale + 160.0).max(600.0);
        let total_h = (layout.bbox.bottom() + 160.0) * scale;
        fixed.set_size_request(total_w as i32, total_h as i32);
        for block in &note.blocks {
            let Some(rect) = layout.rect(&block.id) else {
                continue;
            };
            let widget = blocks::canvas_block_widget(app, block, rect, scale);
            fixed.put(
                &widget,
                (rect.x * scale).max(0.0),
                (rect.y * scale).max(0.0),
            );
        }
        let canvas_scroll = gtk::ScrolledWindow::new();
        canvas_scroll.set_hexpand(true);
        canvas_scroll.set_vexpand(true);
        canvas_scroll.set_child(Some(&fixed));
        let height = (total_h + 60.0).clamp(300.0, 4000.0);
        canvas_scroll.set_min_content_height(height as i32);
        self.editor.append(&canvas_scroll);
    }

    fn refresh_editor_debounced(&self) {
        let app = self.clone_ref();
        glib::idle_add_local_once(move || app.refresh_editor());
    }

    pub fn schedule_save(&self) {
        if let Some(source) = self.autosave.take() {
            source.remove();
        }
        // Snapshot de recuperación inmediato; si el proceso muere antes del
        // autosave, se ofrece al reabrir.
        {
            let state = self.state.borrow();
            if let Some(note) = &state.current_note {
                let _ = folio_store::doctor::write_recovery(&state.store.vault, note);
            }
        }
        let ms = self
            .state
            .borrow()
            .config
            .int("notes.autosave_ms")
            .clamp(200, 5000) as u64;
        let app = self.clone_ref();
        let source = glib::timeout_add_local(Duration::from_millis(ms), move || {
            app.save_now();
            glib::ControlFlow::Break
        });
        self.autosave.set(Some(source));
    }

    pub fn save_now(&self) {
        let result = self.state.borrow_mut().save();
        match result {
            Ok(()) => {
                self.set_status("Guardado");
            }
            Err(error) => self.report(&error),
        }
    }

    pub fn apply_format(&self, tag: &str) {
        let Some(view) = self.focused.borrow().clone() else {
            self.toast("Haz clic en un bloque de texto primero");
            return;
        };
        let buffer = view.buffer();
        blocks::ensure_text_tags(&buffer);
        let (mut start, mut end) = match buffer.selection_bounds() {
            Some(bounds) => bounds,
            None => {
                let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                if !iter.starts_word() {
                    iter.set_line_offset(0);
                }
                let mut start = iter;
                start.forward_to_line_end();
                (iter, start)
            }
        };
        if start == end {
            return;
        }
        let tag_name = match tag {
            "h1" | "h2" | "h3" | "quote" => format!("para-{tag}"),
            "list" => "para-list".into(),
            "checklist" => "para-checklist".into(),
            other => other.to_string(),
        };
        if tag_name.starts_with("para-") {
            if !start.starts_line() {
                start.set_line_offset(0);
            }
            if !end.ends_line() {
                end.forward_to_line_end();
            }
        }
        let has = start.has_tag(
            &buffer
                .tag_table()
                .lookup(&tag_name)
                .unwrap_or_else(|| gtk::TextTag::new(Some("none"))),
        );
        if has {
            buffer.remove_tag_by_name(&tag_name, &start, &end);
        } else {
            buffer.apply_tag_by_name(&tag_name, &start, &end);
        }
    }

    pub fn insert_text_block(&self) {
        let mut state = self.state.borrow_mut();
        if state.current_note.is_none() {
            drop(state);
            self.toast("Crea una nota primero");
            return;
        }
        let block = Block::text_plain("");
        if let Err(error) = state.insert_block(block) {
            drop(state);
            self.report(&error);
            return;
        }
        drop(state);
        self.refresh_editor();
        self.schedule_save();
    }

    pub fn insert_block_kind(&self, kind: &str) {
        let block = match kind {
            "table" => Block::new(
                "table",
                BlockContent::Table(folio_core::model::TableContent::new(3, 3)),
            ),
            "drawing" => Block::new(
                "drawing",
                BlockContent::Drawing(folio_core::model::DrawingContent::default()),
            ),
            "web" => Block::new(
                "web",
                BlockContent::Web(folio_core::model::WebContent {
                    url: "https://example.com".into(),
                    ..Default::default()
                }),
            ),
            "image" => {
                if let Some(block) = self.pick_asset("image") {
                    block
                } else {
                    return;
                }
            }
            "file" => {
                if let Some(block) = self.pick_asset("file") {
                    block
                } else {
                    return;
                }
            }
            "audio" => {
                if let Some(block) = self.pick_asset("audio") {
                    block
                } else {
                    return;
                }
            }
            "video" => {
                if let Some(block) = self.pick_asset("video") {
                    block
                } else {
                    return;
                }
            }
            _ => Block::text_plain(""),
        };
        let mut state = self.state.borrow_mut();
        if let Err(error) = state.insert_block(block) {
            drop(state);
            self.report(&error);
            return;
        }
        drop(state);
        self.refresh_editor();
        self.schedule_save();
    }

    fn pick_asset(&self, kind: &str) -> Option<Block> {
        let dialog = gtk::FileDialog::builder()
            .title(format!("Elegir {kind}"))
            .build();
        let app = self.clone_ref();
        let kind = kind.to_string();
        dialog.open(
            Some(&self.window),
            None::<&gio::Cancellable>,
            move |result| {
                let Ok(file) = result else { return };
                let Some(path) = file.path() else { return };
                let store = app.state.borrow().store.vault.clone();
                let hash = match folio_store::assets::import_file(&store, &path) {
                    Ok(hash) => hash,
                    Err(error) => {
                        app.report(&error);
                        return;
                    }
                };
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                let block = match kind.as_str() {
                    "image" => Block::new(
                        "image",
                        BlockContent::Image(folio_core::model::ImageContent {
                            asset: hash,
                            caption: name,
                            ..Default::default()
                        }),
                    ),
                    "file" => Block::new(
                        "file",
                        BlockContent::File(folio_core::model::FileContent {
                            asset: hash,
                            name,
                            mime: mime_guess(&path),
                            size: std::fs::metadata(&path)
                                .map(|metadata| metadata.len())
                                .unwrap_or(0),
                            ..Default::default()
                        }),
                    ),
                    "audio" => Block::new(
                        "audio",
                        BlockContent::Audio(folio_core::model::AudioContent {
                            asset: hash,
                            title: name,
                            ..Default::default()
                        }),
                    ),
                    "video" => Block::new(
                        "video",
                        BlockContent::Video(folio_core::model::VideoContent {
                            asset: hash,
                            title: name,
                            ..Default::default()
                        }),
                    ),
                    _ => return,
                };
                let mut state = app.state.borrow_mut();
                if let Err(error) = state.insert_block(block) {
                    drop(state);
                    app.report(&error);
                    return;
                }
                drop(state);
                app.refresh_editor();
                app.save_now();
            },
        );
        None
    }

    fn install_config_watch(&self) {
        let root = self.state.borrow().store.root().to_path_buf();
        let mut paths = vec![root.join("vault.toml")];
        let global = folio_config::global_config_path();
        if global != root.join("vault.toml") {
            paths.push(global);
        }
        for path in paths {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let file = gio::File::for_path(&path);
            let Ok(monitor) =
                file.monitor_file(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)
            else {
                continue;
            };
            let app = self.clone_ref();
            let root = root.clone();
            monitor.connect_changed(move |_, _, _, event| {
                if event == gio::FileMonitorEvent::Deleted {
                    return;
                }
                match crate::helpers::load_config(Some(&root)) {
                    Ok(config) => {
                        apply_theme(&config);
                        app.state.borrow_mut().config = config;
                        app.refresh_sidebar_inner_public();
                        app.refresh_editor();
                        app.set_status("Configuración recargada");
                    }
                    Err(error) => {
                        app.state.borrow_mut().config_error = Some(error.clone());
                        app.toast(&format!(
                            "Configuración inválida; se conserva la anterior: {error}"
                        ));
                    }
                }
            });
            self.config_monitors.borrow_mut().push(monitor);
        }
    }

    fn install_day_watch(&self) {
        let app = self.clone_ref();
        glib::timeout_add_seconds_local(60, move || {
            let (enabled, create) = {
                let state = app.state.borrow();
                let daily = crate::helpers::daily_config(&state.config);
                (daily.enabled, daily.create.clone())
            };
            if enabled && create == "first_open" {
                let before = {
                    let state = app.state.borrow();
                    crate::helpers::today_date(&state.config, &state.clock)
                        .ok()
                        .and_then(|date| state.store.daily_book(&date).ok().flatten())
                        .map(|book| book.meta.id)
                };
                let result = {
                    let state = app.state.borrow();
                    crate::helpers::ensure_today(&state.store, &state.config, &state.clock, true)
                };
                if let Ok(book) = result
                    && before.as_deref() != Some(book.meta.id.as_str())
                {
                    app.refresh_sidebar_inner_public();
                    app.toast(&format!("Nuevo libro del día: {}", book.meta.title));
                }
            }
            glib::ControlFlow::Continue
        });
    }

    pub fn clone_ref(&self) -> Rc<App> {
        self.self_ref
            .borrow()
            .upgrade()
            .expect("App vive mientras sus callbacks")
    }
}

/// Aplica el tema del esquema al gestor de estilo de Libadwaita.
pub fn apply_theme(config: &folio_config::Config) {
    let manager = adw::StyleManager::default();
    match config.string("ui.theme").as_str() {
        "light" => manager.set_color_scheme(adw::ColorScheme::ForceLight),
        "dark" => manager.set_color_scheme(adw::ColorScheme::ForceDark),
        _ => manager.set_color_scheme(adw::ColorScheme::Default),
    }
}

fn mime_guess(path: &std::path::Path) -> String {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "txt" | "md" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn connect_widgets(
    app: &Rc<App>,
    new_note: &gtk::Button,
    new_book: &gtk::Button,
    add_note: &gtk::Button,
    today: &gtk::Button,
) {
    let app_new = app.clone();
    new_note.connect_clicked(move |_| {
        let _ = gtk::prelude::WidgetExt::activate_action(&app_new.window, "new-note", None);
    });
    let app_add = app.clone();
    add_note.connect_clicked(move |_| {
        let _ = gtk::prelude::WidgetExt::activate_action(&app_add.window, "new-note", None);
    });
    let app_book = app.clone();
    new_book.connect_clicked(move |_| {
        let _ = gtk::prelude::WidgetExt::activate_action(&app_book.window, "new-book", None);
    });
    let app_today = app.clone();
    today.connect_clicked(move |_| {
        let _ = gtk::prelude::WidgetExt::activate_action(&app_today.window, "today", None);
    });

    let app_search = app.clone();
    app.search_entry.connect_search_changed(move |entry| {
        let text = entry.text().to_string();
        let _ = app_search.state.borrow_mut().search(&text);
        app_search.refresh_notes_inner_public();
    });

    let app_title = app.clone();
    app.title_entry.connect_changed(move |entry| {
        if app_title.suppress_title.get() {
            return;
        }
        let text = entry.text().to_string();
        {
            let mut state = app_title.state.borrow_mut();
            let clock = state.clock;
            if let Some(note) = state.current_note.as_mut() {
                note.title = text;
                note.touch(&clock);
                state.dirty = true;
            }
        }
        app_title.schedule_save();
    });
}

fn setup_actions(app: &Rc<App>) {
    let window = &app.window;
    install_action(app, "new-note", |app| {
        let result = app.state.borrow_mut().create_note();
        match result {
            Ok(_) => {
                app.refresh_all();
                app.refresh_editor();
            }
            Err(error) => app.report(&error),
        }
    });
    install_action(app, "new-book", |app| {
        dialogs::prompt_text(app, "Nuevo libro", "Título del libro", "", |app, title| {
            if title.trim().is_empty() {
                return;
            }
            let result = app.state.borrow_mut().create_book(title.trim());
            match result {
                Ok(_) => app.refresh_all(),
                Err(error) => app.report(&error),
            }
        });
    });
    install_action(app, "today", |app| {
        let result = app.state.borrow_mut().today();
        match result {
            Ok(_) => app.refresh_all(),
            Err(error) => app.report(&error),
        }
    });
    install_action(app, "search", |app| {
        app.search_entry.grab_focus();
    });
    install_action(app, "palette", dialogs::open_palette);
    install_action(app, "settings", dialogs::open_settings);
    install_action(app, "plugins", dialogs::open_plugins);
    install_action(app, "trash", dialogs::open_trash);
    install_action(app, "move-note", dialogs::open_move);
    install_action(app, "rename-book", |app| {
        let title = app
            .state
            .borrow()
            .current_book()
            .map(|book| book.meta.title)
            .unwrap_or_default();
        dialogs::prompt_text(app, "Renombrar libro", "Título", &title, |app, title| {
            if title.trim().is_empty() {
                return;
            }
            if let Err(error) = app.state.borrow_mut().rename_current_book(title.trim()) {
                app.report(&error);
            }
            app.refresh_all();
        });
    });
    install_action(app, "delete-note", |app| {
        let confirm = gtk::AlertDialog::builder()
            .message("¿Mover la nota a la papelera?")
            .detail("Podrás restaurarla durante 30 días.")
            .buttons(["Cancelar", "Mover a papelera"])
            .cancel_button(0)
            .default_button(0)
            .build();
        let app = app.clone();
        let parent = app.window.clone();
        confirm.choose(Some(&parent), None::<&gio::Cancellable>, move |result| {
            if result != Ok(1) {
                return;
            }
            match app.state.borrow_mut().delete_current_note() {
                Ok(_) => {
                    app.refresh_all();
                    app.toast("Nota movida a la papelera");
                }
                Err(error) => app.report(&error),
            }
        });
    });
    install_action(app, "export-md", |app| export_dialog(app, "md"));
    install_action(app, "export-html", |app| export_dialog(app, "html"));
    install_action(app, "import-md", |app| {
        let dialog = gtk::FileDialog::builder()
            .title("Importar Markdown")
            .build();
        let app = app.clone();
        let parent = app.window.clone();
        dialog.open(Some(&parent), None::<&gio::Cancellable>, move |result| {
            let Ok(file) = result else { return };
            let Some(path) = file.path() else { return };
            match app.state.borrow_mut().import_markdown(&path) {
                Ok(_) => {
                    app.refresh_all();
                    app.toast("Markdown importado");
                }
                Err(error) => app.report(&error),
            }
        });
    });
    install_action(app, "backup", |app| {
        match app.state.borrow_mut().create_backup() {
            Ok(path) => app.toast(&format!("Backup creado: {}", path.display())),
            Err(error) => app.report(&error),
        }
    });
    install_action(app, "preview", |app| {
        let mode = {
            let mut state = app.state.borrow_mut();
            state.mode = if state.mode == ViewMode::Preview {
                ViewMode::Document
            } else {
                ViewMode::Preview
            };
            state.mode
        };
        app.refresh_editor();
        app.set_status(match mode {
            ViewMode::Preview => "Previsualización de solo lectura",
            _ => "Documento",
        });
    });
    install_action(app, "canvas", |app| {
        let mode = {
            let mut state = app.state.borrow_mut();
            state.mode = if state.mode == ViewMode::Canvas {
                ViewMode::Document
            } else {
                ViewMode::Canvas
            };
            state.mode
        };
        app.refresh_editor();
        app.set_status(match mode {
            ViewMode::Canvas => "Lienzo: arrastra un bloque para anclarlo",
            _ => "Documento",
        });
    });
    install_action(app, "recompose", |app| {
        if let Err(error) = app.state.borrow_mut().recompose() {
            app.report(&error);
        }
        app.refresh_editor();
        app.toast("Bloques recompuestos");
    });
    install_action(app, "fit-document", |app| {
        app.state.borrow_mut().canvas_scale = 1.0;
        app.refresh_editor();
        app.set_status("Encuadre documento al 100 %");
    });
    install_action(app, "fit-all", |app| {
        app.state.borrow_mut().canvas_scale = 0.6;
        app.refresh_editor();
        app.set_status("Encuadre todo al 60 %");
    });
    install_action(app, "zoom-in", |app| {
        let mut state = app.state.borrow_mut();
        state.canvas_scale = (state.canvas_scale * 1.2).min(4.0);
        drop(state);
        app.refresh_editor();
    });
    install_action(app, "zoom-out", |app| {
        let mut state = app.state.borrow_mut();
        state.canvas_scale = (state.canvas_scale / 1.2).max(0.25);
        drop(state);
        app.refresh_editor();
    });
    install_action(app, "zoom-reset", |app| {
        app.state.borrow_mut().canvas_scale = 1.0;
        app.refresh_editor();
    });
    install_action(app, "add-text-block", |app| app.insert_text_block());
    for kind in ["table", "drawing", "image", "file", "audio", "video", "web"] {
        let action = format!("add-{kind}");
        install_action(app, &action, move |app| {
            app.insert_block_kind(kind);
        });
    }
    for tag in ["bold", "italic", "underline", "strikethrough", "code"] {
        let action = format!("format-{tag}");
        install_action(app, &action, move |app| app.apply_format(tag));
    }
    for tag in ["h1", "h2", "h3", "quote", "list", "checklist"] {
        let action = format!("style-{tag}");
        install_action(app, &action, move |app| app.apply_format(tag));
    }
    install_action(app, "format-link", |app| {
        dialogs::prompt_text(app, "Enlace", "URL", "https://", |app, url| {
            if folio_core::validate::validate_url(url).is_err() {
                app.toast("URL no permitida");
                return;
            }
            let Some(view) = app.focused.borrow().clone() else {
                app.toast("Selecciona texto primero");
                return;
            };
            let buffer = view.buffer();
            let tag_name = format!("link:{url}");
            if buffer.tag_table().lookup(&tag_name).is_none() {
                let tag = gtk::TextTag::new(Some(&tag_name));
                tag.set_underline(gtk::pango::Underline::Single);
                tag.set_foreground(Some("#3584e4"));
                buffer.tag_table().add(&tag);
            }
            if let Some((start, end)) = buffer.selection_bounds() {
                buffer.apply_tag_by_name(&tag_name, &start, &end);
            }
        });
    });
    install_action(app, "about", |app| {
        let dialog = gtk::AboutDialog::builder()
            .program_name("Folio")
            .version(env!("CARGO_PKG_VERSION"))
            .comments("Notas y libros diarios, local primero.")
            .website("https://github.com/leonisauro/folio")
            .build();
        dialog.set_transient_for(Some(&app.window));
        dialog.set_modal(true);
        dialog.present();
    });
    install_action(app, "quit", |app| app.window.close());
    install_action(app, "undo", |app| {
        let mut state = app.state.borrow_mut();
        let current = state.current_note.clone();
        if let Some(current) = current
            && let Some(previous) = state.undo.undo(&current)
        {
            state.current_note = Some(previous);
            state.dirty = true;
            drop(state);
            app.refresh_editor();
            app.schedule_save();
        }
    });
    install_action(app, "redo", |app| {
        let mut state = app.state.borrow_mut();
        let current = state.current_note.clone();
        if let Some(current) = current
            && let Some(next) = state.undo.redo(&current)
        {
            state.current_note = Some(next);
            state.dirty = true;
            drop(state);
            app.refresh_editor();
            app.schedule_save();
        }
    });
    install_action(app, "toggle-grid", |app| {
        let mut state = app.state.borrow_mut();
        if let Some(note) = state.current_note.as_mut() {
            note.layout.show_grid = !note.layout.show_grid;
            state.dirty = true;
        }
        drop(state);
        app.refresh_editor();
        app.schedule_save();
    });
    let _ = window;

    for (action, accel) in [
        ("new-note", "<Control>n"),
        ("new-book", "<Control><Shift>n"),
        ("palette", "<Control>k"),
        ("search", "<Control>p"),
        ("today", "<Control><Shift>h"),
        ("preview", "<Control><Shift>e"),
        ("move-note", "<Control><Shift>m"),
        ("fit-document", "<Control>1"),
        ("fit-all", "<Control>2"),
        ("zoom-in", "<Control>plus"),
        ("zoom-out", "<Control>minus"),
        ("zoom-reset", "<Control>0"),
        ("format-bold", "<Control>b"),
        ("format-italic", "<Control>i"),
        ("format-underline", "<Control>u"),
        ("format-strikethrough", "<Control><Shift>x"),
        ("format-code", "<Control><Shift>c"),
        ("settings", "<Control>comma"),
        ("undo", "<Control>z"),
        ("redo", "<Control><Shift>z"),
        ("delete-note", "<Control>Delete"),
    ] {
        app.application
            .set_accels_for_action(&format!("win.{action}"), &[accel]);
    }
}

fn install_action<F>(app: &Rc<App>, name: &str, callback: F)
where
    F: Fn(&Rc<App>) + 'static,
{
    let action = gio::SimpleAction::new(name, None);
    let app_clone = app.clone();
    action.connect_activate(move |_, _| callback(&app_clone));
    app.window.add_action(&action);
}

fn export_dialog(app: &Rc<App>, format: &str) {
    let name = app
        .state
        .borrow()
        .current_note
        .as_ref()
        .map(|note| note.display_title())
        .unwrap_or_else(|| "nota".into());
    let dialog = gtk::FileDialog::builder()
        .title(format!("Exportar a {format}"))
        .initial_name(format!("{}.{format}", sanitize_name(&name)))
        .build();
    let app = app.clone();
    let parent = app.window.clone();
    let format = format.to_string();
    dialog.save(Some(&parent), None::<&gio::Cancellable>, move |result| {
        let Ok(file) = result else { return };
        let Some(path) = file.path() else { return };
        match app.state.borrow_mut().export_current(&format, &path) {
            Ok(path) => app.toast(&format!("Exportado: {}", path.display())),
            Err(error) => app.report(&error),
        }
    });
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(60)
        .collect::<String>()
}

fn build_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    let file = gio::Menu::new();
    file.append(Some("Nueva nota"), Some("win.new-note"));
    file.append(Some("Nuevo libro"), Some("win.new-book"));
    file.append(Some("Ir a hoy"), Some("win.today"));
    file.append(Some("Mover nota a…"), Some("win.move-note"));
    file.append(Some("Renombrar libro"), Some("win.rename-book"));
    menu.append_section(None, &file);

    let edit = gio::Menu::new();
    edit.append(Some("Deshacer"), Some("win.undo"));
    edit.append(Some("Rehacer"), Some("win.redo"));
    edit.append(Some("Paleta de comandos"), Some("win.palette"));
    edit.append(Some("Buscar"), Some("win.search"));
    menu.append_section(None, &edit);

    let view = gio::Menu::new();
    view.append(Some("Previsualizar"), Some("win.preview"));
    view.append(Some("Lienzo"), Some("win.canvas"));
    view.append(Some("Recomponer"), Some("win.recompose"));
    view.append(Some("Encuadre documento"), Some("win.fit-document"));
    view.append(Some("Encuadre todo"), Some("win.fit-all"));
    view.append(Some("Rejilla"), Some("win.toggle-grid"));
    menu.append_section(None, &view);

    let data = gio::Menu::new();
    data.append(Some("Exportar Markdown"), Some("win.export-md"));
    data.append(Some("Exportar HTML"), Some("win.export-html"));
    data.append(Some("Importar Markdown"), Some("win.import-md"));
    data.append(Some("Crear backup"), Some("win.backup"));
    data.append(Some("Papelera"), Some("win.trash"));
    menu.append_section(None, &data);

    let tools = gio::Menu::new();
    tools.append(Some("Plugins"), Some("win.plugins"));
    tools.append(Some("Ajustes"), Some("win.settings"));
    tools.append(Some("Eliminar nota"), Some("win.delete-note"));
    tools.append(Some("Acerca de Folio"), Some("win.about"));
    tools.append(Some("Salir"), Some("win.quit"));
    menu.append_section(None, &tools);
    menu
}

fn build_block_menu() -> gio::Menu {
    let menu = gio::Menu::new();
    menu.append(Some("Texto"), Some("win.add-text-block"));
    menu.append(Some("Tabla"), Some("win.add-table"));
    menu.append(Some("Dibujo"), Some("win.add-drawing"));
    menu.append(Some("Imagen"), Some("win.add-image"));
    menu.append(Some("Archivo"), Some("win.add-file"));
    menu.append(Some("Audio"), Some("win.add-audio"));
    menu.append(Some("Vídeo"), Some("win.add-video"));
    menu.append(Some("Web"), Some("win.add-web"));
    menu
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(source) = self.autosave.take() {
            source.remove();
        }
    }
}
