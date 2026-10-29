use std::cell::{Cell, RefCell};
use std::rc::Rc;

use folio_config::{FieldType, Scope};
use folio_core::Clock;
use gtk::prelude::*;

use crate::window::App;

pub fn prompt_text<F>(app: &Rc<App>, title: &str, placeholder: &str, initial: &str, callback: F)
where
    F: Fn(&Rc<App>, &str) + 'static,
{
    let window = gtk::Window::builder()
        .title(title)
        .default_width(420)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 12);
    box_.set_margin_top(16);
    box_.set_margin_bottom(16);
    box_.set_margin_start(16);
    box_.set_margin_end(16);
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.set_text(initial);
    entry.set_activates_default(true);
    box_.append(&entry);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancelar");
    let accept = gtk::Button::with_label("Aceptar");
    accept.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&accept);
    box_.append(&buttons);
    window.set_child(Some(&box_));
    let window_cancel = window.clone();
    cancel.connect_clicked(move |_| window_cancel.close());
    let app_accept = app.clone();
    let window_accept = window.clone();
    let entry_accept = entry.clone();
    let callback = Rc::new(callback);
    accept.connect_clicked(move |_| {
        callback(&app_accept, &entry_accept.text());
        window_accept.close();
    });
    let app_enter = app.clone();
    let window_enter = window.clone();
    let callback_enter = Rc::new(move |app: &Rc<App>, text: &str| {
        // El callback se guarda de nuevo en un Rc para el gesto de Enter.
        let _ = (app, text);
    });
    let _ = (app_enter, window_enter, callback_enter);
    window.present();
    entry.grab_focus();
}

pub fn open_settings(app: &Rc<App>) {
    let window = gtk::Window::builder()
        .title("Ajustes de Folio")
        .default_width(780)
        .default_height(680)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_margin_top(10);
    root.set_margin_bottom(10);
    root.set_margin_start(12);
    root.set_margin_end(12);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Buscar ajuste por nombre, ruta o grupo"));
    root.append(&search);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_vexpand(true);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    scroller.set_child(Some(&content));
    root.append(&scroller);
    window.set_child(Some(&root));

    let updating = Rc::new(Cell::new(false));
    let rows: Rc<RefCell<Vec<(String, gtk::Widget)>>> = Rc::new(RefCell::new(Vec::new()));
    let values = app.state.borrow().config.clone();
    let mut groups: std::collections::BTreeMap<String, Vec<folio_config::Field>> =
        std::collections::BTreeMap::new();
    for field in folio_config::schema() {
        groups.entry(field.group.clone()).or_default().push(field);
    }
    for (group, fields) in groups {
        let expander = gtk::Expander::new(Some(&group));
        expander.set_expanded(true);
        let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        list.set_margin_start(8);
        list.set_margin_top(4);
        for field in fields {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.set_margin_top(2);
            row.set_margin_bottom(2);
            let label = gtk::Label::new(Some(&field.label));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            label.set_tooltip_text(Some(&format!("{} · {}", field.path, field.help)));
            row.append(&label);
            let value = values
                .get(&field.path)
                .cloned()
                .unwrap_or(field.default.clone());
            let widget = setting_widget(app, &field, &value, &updating);
            row.append(&widget);
            let scope = gtk::Label::new(Some(match field.scope {
                Scope::Vault => "bóveda",
                Scope::Global => "global",
                Scope::Both => "ambos",
            }));
            scope.add_css_class("caption");
            row.append(&scope);
            list.append(&row);
            rows.borrow_mut().push((
                format!(
                    "{} {} {} {}",
                    field.label, field.path, field.group, field.help
                )
                .to_lowercase(),
                row.clone().upcast(),
            ));
        }
        expander.set_child(Some(&list));
        content.append(&expander);
    }
    let rows_filter = rows.clone();
    search.connect_search_changed(move |entry| {
        let query = entry.text().to_lowercase();
        for (text, row) in rows_filter.borrow().iter() {
            row.set_visible(query.is_empty() || text.contains(&query));
        }
    });
    window.present();
}

fn setting_widget(
    app: &Rc<App>,
    field: &folio_config::Field,
    value: &serde_json::Value,
    updating: &Rc<Cell<bool>>,
) -> gtk::Widget {
    match field.ty {
        FieldType::Bool => {
            let switch = gtk::Switch::new();
            switch.set_active(value.as_bool().unwrap_or(false));
            switch.set_valign(gtk::Align::Center);
            let app = app.clone();
            let key = field.path.clone();
            let updating = updating.clone();
            switch.connect_active_notify(move |switch| {
                if updating.get() {
                    return;
                }
                if let Err(error) = app
                    .state
                    .borrow_mut()
                    .set_config(&key, if switch.is_active() { "true" } else { "false" })
                {
                    app.report_str(error);
                }
            });
            switch.upcast()
        }
        FieldType::Int | FieldType::Float => {
            let is_float = field.ty == FieldType::Float;
            let min = field.min.unwrap_or(0.0);
            let max = field.max.unwrap_or(1_000_000.0);
            let value = if is_float {
                value.as_f64().unwrap_or(min)
            } else {
                value.as_i64().unwrap_or(min as i64) as f64
            };
            let adjustment =
                gtk::Adjustment::new(value, min, max, if is_float { 0.1 } else { 1.0 }, 10.0, 0.0);
            let spin = gtk::SpinButton::new(
                Some(&adjustment),
                if is_float { 0.1 } else { 1.0 },
                if is_float { 2 } else { 0 },
            );
            spin.set_valign(gtk::Align::Center);
            spin.set_width_chars(8);
            let app = app.clone();
            let key = field.path.clone();
            let updating = updating.clone();
            spin.connect_value_changed(move |spin| {
                if updating.get() {
                    return;
                }
                let raw = if is_float {
                    format!("{}", spin.value())
                } else {
                    format!("{}", spin.value_as_int())
                };
                if let Err(error) = app.state.borrow_mut().set_config(&key, &raw) {
                    app.report_str(error);
                }
            });
            spin.upcast()
        }
        FieldType::Enum => {
            let choices: Vec<&str> = field.choices.iter().map(String::as_str).collect();
            let list = gtk::StringList::new(&choices);
            let dropdown = gtk::DropDown::new(Some(list), gtk::Expression::NONE);
            let selected = value.as_str().unwrap_or_default();
            let index = field
                .choices
                .iter()
                .position(|choice| choice == selected)
                .unwrap_or(0) as u32;
            dropdown.set_selected(index);
            dropdown.set_valign(gtk::Align::Center);
            let app = app.clone();
            let key = field.path.clone();
            let choices = field.choices.clone();
            let updating = updating.clone();
            dropdown.connect_selected_notify(move |dropdown| {
                if updating.get() {
                    return;
                }
                if let Some(choice) = choices.get(dropdown.selected() as usize)
                    && let Err(error) = app.state.borrow_mut().set_config(&key, choice)
                {
                    app.report_str(error);
                }
            });
            dropdown.upcast()
        }
        _ => {
            let entry = gtk::Entry::new();
            entry.set_text(value.as_str().unwrap_or_default());
            entry.set_width_chars(24);
            entry.set_valign(gtk::Align::Center);
            let app = app.clone();
            let key = field.path.clone();
            let updating = updating.clone();
            entry.connect_changed(move |entry| {
                if updating.get() {
                    return;
                }
                if let Err(error) = app.state.borrow_mut().set_config(&key, &entry.text()) {
                    app.report_str(error);
                }
            });
            entry.upcast()
        }
    }
}

pub fn open_palette(app: &Rc<App>) {
    let window = gtk::Window::builder()
        .title("Paleta de comandos")
        .default_width(640)
        .default_height(520)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
    box_.set_margin_top(8);
    box_.set_margin_bottom(8);
    box_.set_margin_start(8);
    box_.set_margin_end(8);
    let entry = gtk::SearchEntry::new();
    entry.set_placeholder_text(Some("Comando o nota…"));
    box_.append(&entry);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_vexpand(true);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    scroller.set_child(Some(&list));
    box_.append(&scroller);
    window.set_child(Some(&box_));

    let commands: Vec<(&str, &str, &str)> = vec![
        ("Nueva nota", "new-note", "Ctrl+N"),
        ("Nuevo libro", "new-book", "Ctrl+Shift+N"),
        ("Ir a hoy", "today", "Ctrl+Shift+H"),
        ("Buscar", "search", "Ctrl+P"),
        ("Ajustes", "settings", "Ctrl+,"),
        ("Plugins", "plugins", ""),
        ("Papelera", "trash", ""),
        ("Exportar HTML", "export-html", ""),
        ("Exportar Markdown", "export-md", ""),
        ("Importar Markdown", "import-md", ""),
        ("Previsualizar", "preview", "Ctrl+Shift+E"),
        ("Lienzo", "canvas", ""),
        ("Recomponer", "recompose", ""),
        ("Crear backup", "backup", ""),
    ];
    let entries: Rc<RefCell<Vec<(String, String, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let entries_fill = entries.clone();
    for (label, action, accel) in &commands {
        entries_fill
            .borrow_mut()
            .push(((*label).to_string(), format!("action:{action}"), false));
        let _ = accel;
    }

    let window_fill = window.clone();
    let app_fill = app.clone();
    let list_fill = list.clone();
    let populate: Rc<dyn Fn(&str)> = {
        let entries = entries.clone();
        let app = app_fill.clone();
        let list = list_fill.clone();
        let window = window_fill.clone();
        Rc::new(move |query: &str| {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let query = query.to_lowercase();
            let mut items: Vec<(String, String, bool)> = entries
                .borrow()
                .iter()
                .filter(|(label, _, _)| label.to_lowercase().contains(&query))
                .cloned()
                .collect();
            // Notas: búsqueda FTS en toda la bóveda.
            if !query.is_empty() {
                let hits = app
                    .state
                    .borrow()
                    .index
                    .search(&query, 20)
                    .unwrap_or_default();
                for hit in hits {
                    items.push((hit.title, format!("note:{}", hit.note_id), true));
                }
            }
            let app_open = app.clone();
            let window_open = window.clone();
            let list_for_activate = list.clone();
            for (label, target, is_note) in items {
                let row = gtk::ListBoxRow::new();
                let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                box_.set_margin_top(6);
                box_.set_margin_bottom(6);
                let text = gtk::Label::new(Some(&label));
                text.set_xalign(0.0);
                text.set_hexpand(true);
                box_.append(&text);
                let kind = gtk::Label::new(Some(if is_note { "nota" } else { "comando" }));
                kind.add_css_class("caption");
                box_.append(&kind);
                row.set_child(Some(&box_));
                let app = app_open.clone();
                let window = window_open.clone();
                let target = target.clone();
                let list_for_row = list_for_activate.clone();
                row.connect_activate(move |_| {
                    if let Some(note_id) = target.strip_prefix("note:") {
                        if let Err(error) = app.state.borrow_mut().open_note(note_id) {
                            app.report(&error);
                        }
                        app.refresh_all();
                    } else if let Some(action) = target.strip_prefix("action:") {
                        let _ = gtk::prelude::WidgetExt::activate_action(&app.window, action, None);
                    }
                    window.close();
                    let _ = &list_for_row;
                });
                list.append(&row);
            }
            if let Some(first) = list.row_at_index(0) {
                list.select_row(Some(&first));
            }
        })
    };
    let populate_clone = populate.clone();
    entry.connect_search_changed(move |entry| populate_clone(&entry.text()));
    let populate_enter = populate.clone();
    let list_enter = list.clone();
    let window_enter = window.clone();
    entry.connect_activate(move |entry| {
        populate_enter(&entry.text());
        if let Some(row) = list_enter.row_at_index(0) {
            row.emit_activate();
        }
        let _ = &window_enter;
    });
    populate("");
    window.present();
    entry.grab_focus();
}

pub fn open_move(app: &Rc<App>) {
    let note = app.state.borrow().current_note.clone();
    let Some(note) = note else {
        app.toast("Abre una nota primero");
        return;
    };
    let books: Vec<folio_store::Book> = app
        .state
        .borrow()
        .books()
        .unwrap_or_default()
        .into_iter()
        .filter(|book| !book.meta.archived)
        .collect();
    let window = gtk::Window::builder()
        .title("Mover nota")
        .default_width(460)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 10);
    box_.set_margin_top(14);
    box_.set_margin_bottom(14);
    box_.set_margin_start(14);
    box_.set_margin_end(14);
    let book_label = gtk::Label::new(Some("Libro de destino"));
    book_label.set_xalign(0.0);
    box_.append(&book_label);
    let names: Vec<&str> = books.iter().map(|book| book.meta.title.as_str()).collect();
    let book_list = gtk::StringList::new(&names);
    let book_dropdown = gtk::DropDown::new(Some(book_list), gtk::Expression::NONE);
    box_.append(&book_dropdown);
    let position_label = gtk::Label::new(Some("Posición"));
    position_label.set_xalign(0.0);
    box_.append(&position_label);
    let positions = gtk::StringList::new(&["Al final", "Al principio", "Antes de una nota"]);
    let position_dropdown = gtk::DropDown::new(Some(positions), gtk::Expression::NONE);
    box_.append(&position_dropdown);
    let notes_label = gtk::Label::new(Some("Nota de referencia"));
    notes_label.set_xalign(0.0);
    box_.append(&notes_label);
    let note_list = gtk::StringList::new(&[]);
    let note_dropdown = gtk::DropDown::new(Some(note_list.clone()), gtk::Expression::NONE);
    note_dropdown.set_sensitive(false);
    box_.append(&note_dropdown);

    let all_notes: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
    let app_books = app.clone();
    let all_notes_fill = all_notes.clone();
    let note_list_fill = note_list.clone();
    let note_dropdown_fill = note_dropdown.clone();
    let note_id_for_list = note.id.clone();
    let update_notes: Rc<dyn Fn(u32)> = Rc::new(move |index: u32| {
        let Some(book) = app_books
            .state
            .borrow()
            .books()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| !b.meta.archived)
            .nth(index as usize)
        else {
            return;
        };
        let notes = app_books
            .state
            .borrow()
            .store
            .notes(&book)
            .unwrap_or_default();
        *all_notes_fill.borrow_mut() = notes
            .iter()
            .filter(|summary| summary.id != note_id_for_list)
            .map(|summary| (summary.id.clone(), summary.title.clone()))
            .collect();
        let labels: Vec<String> = all_notes_fill
            .borrow()
            .iter()
            .map(|(_, title)| title.clone())
            .collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        note_list_fill.splice(0, note_list_fill.n_items(), &label_refs);
        note_dropdown_fill.set_sensitive(!labels.is_empty());
    });
    let update_notes_clone = update_notes.clone();
    book_dropdown.connect_selected_notify(move |dropdown| update_notes_clone(dropdown.selected()));
    let update_notes_pos = update_notes.clone();
    let book_dropdown_for_pos = book_dropdown.clone();
    position_dropdown.connect_selected_notify(move |dropdown| {
        update_notes_pos(book_dropdown_for_pos.selected());
        let _ = dropdown;
    });
    update_notes(book_dropdown.selected());

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancelar");
    let accept = gtk::Button::with_label("Mover");
    accept.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&accept);
    box_.append(&buttons);
    window.set_child(Some(&box_));
    let window_cancel = window.clone();
    cancel.connect_clicked(move |_| window_cancel.close());

    let app_move = app.clone();
    let window_move = window.clone();
    let note_id = note.id.clone();
    let all_notes_end = all_notes.clone();
    let book_dropdown_end = book_dropdown.clone();
    accept.connect_clicked(move |_| {
        let index = book_dropdown_end.selected();
        let Some(book) = app_move
            .state
            .borrow()
            .books()
            .unwrap_or_default()
            .into_iter()
            .filter(|book| !book.meta.archived)
            .nth(index as usize)
        else {
            return;
        };
        let before = if position_dropdown.selected() == 2 {
            all_notes_end
                .borrow()
                .get(note_dropdown.selected() as usize)
                .map(|(id, _)| id.clone())
        } else {
            None
        };
        let result = if position_dropdown.selected() == 1 {
            app_move
                .state
                .borrow_mut()
                .move_note_start(&note_id, &book.meta.id)
        } else {
            app_move
                .state
                .borrow_mut()
                .move_note(&note_id, &book.meta.id, before.as_deref())
        };
        match result {
            Ok(()) => {
                app_move.refresh_all();
                app_move.toast("Nota movida");
                window_move.close();
            }
            Err(error) => app_move.report(&error),
        }
    });
    window.present();
}

pub fn open_plugins(app: &Rc<App>) {
    let window = gtk::Window::builder()
        .title("Plugins")
        .default_width(720)
        .default_height(560)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 8);
    box_.set_margin_top(10);
    box_.set_margin_bottom(10);
    box_.set_margin_start(10);
    box_.set_margin_end(10);
    let intro = gtk::Label::new(Some(
        "Los plugins declaran permisos y se ejecutan aislados. Sin red por defecto.",
    ));
    intro.set_xalign(0.0);
    intro.add_css_class("caption");
    box_.append(&intro);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_vexpand(true);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    scroller.set_child(Some(&list));
    box_.append(&scroller);

    let state = app.state.borrow();
    let manifests = state.registry.available_manifests(state.store.root());
    let installed = state.registry.plugins.clone();
    drop(state);
    for manifest in manifests {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("file-card");
        let info = gtk::Box::new(gtk::Orientation::Vertical, 2);
        info.set_hexpand(true);
        let title = gtk::Label::new(Some(&format!("{} · {}", manifest.name, manifest.version)));
        title.set_xalign(0.0);
        title.add_css_class("plugin-title");
        let detail = gtk::Label::new(Some(&format!(
            "{} · permisos: {}",
            manifest.id,
            if manifest.permissions.is_empty() {
                "ninguno".into()
            } else {
                format!("{:?}", manifest.permissions)
            }
        )));
        detail.set_xalign(0.0);
        detail.add_css_class("caption");
        info.append(&title);
        info.append(&detail);
        row.append(&info);
        if let Some(block) = manifest.blocks.first() {
            let insert = gtk::Button::with_label("Insertar");
            let app = app.clone();
            let plugin = manifest.id.clone();
            let block_id = block.id.clone();
            let window = window.clone();
            insert.connect_clicked(move |_| {
                if let Err(error) = app
                    .state
                    .borrow_mut()
                    .insert_plugin_block(&plugin, &block_id)
                {
                    app.report_str(error);
                    return;
                }
                app.refresh_editor();
                app.save_now();
                window.close();
            });
            row.append(&insert);
        }
        if let Some(entry) = installed.get(&manifest.id) {
            let switch = gtk::Switch::new();
            switch.set_active(entry.enabled);
            let app_switch = app.clone();
            let id = manifest.id.clone();
            switch.connect_active_notify(move |switch| {
                let mut state = app_switch.state.borrow_mut();
                let _ = state.registry.set_enabled(&id, switch.is_active());
                let root = state.store.root().to_path_buf();
                let _ = state.registry.save(&root);
                drop(state);
                app_switch.refresh_editor();
            });
            row.append(&switch);
            let uninstall = gtk::Button::with_label("Desinstalar");
            let app = app.clone();
            let id = manifest.id.clone();
            let window = window.clone();
            uninstall.connect_clicked(move |_| {
                let mut state = app.state.borrow_mut();
                let root = state.store.root().to_path_buf();
                if let Err(error) = state.registry.uninstall(&root, &id) {
                    drop(state);
                    app.report_str(error);
                    return;
                }
                drop(state);
                app.refresh_editor();
                window.close();
            });
            row.append(&uninstall);
        }
        list.append(&row);
    }
    let install = gtk::Button::with_label("Instalar desde carpeta…");
    let app_install = app.clone();
    install.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title("Elegir paquete de plugin")
            .build();
        let app = app_install.clone();
        let parent = app.window.clone();
        dialog.select_folder(Some(&parent), None::<&gio::Cancellable>, move |result| {
            let Ok(file) = result else { return };
            let Some(path) = file.path() else { return };
            let mut state = app.state.borrow_mut();
            let allow = state.config.bool("plugins.allow_unsigned");
            let installed_at = state.clock.now_rfc3339();
            let root = state.store.root().to_path_buf();
            let result = state
                .registry
                .install_from_dir(&root, &path, allow, None, &installed_at);
            drop(state);
            match result {
                Ok(plugin) => app.toast(&format!("{} instalado", plugin.id)),
                Err(error) => app.report_str(error),
            }
        });
    });
    box_.append(&install);
    window.set_child(Some(&box_));
    window.present();
}

pub fn open_trash(app: &Rc<App>) {
    let window = gtk::Window::builder()
        .title("Papelera")
        .default_width(620)
        .default_height(480)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
    box_.set_margin_top(10);
    box_.set_margin_bottom(10);
    box_.set_margin_start(10);
    box_.set_margin_end(10);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_vexpand(true);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    scroller.set_child(Some(&list));
    box_.append(&scroller);
    let entries = {
        let state = app.state.borrow();
        folio_store::trash::list(&state.store.vault, state.store.crypto()).unwrap_or_default()
    };
    if entries.is_empty() {
        list.append(&gtk::Label::new(Some("La papelera está vacía.")));
    }
    for entry in entries {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("file-card");
        let title = gtk::Label::new(Some(&format!(
            "{} · {} · {}",
            entry.meta.kind, entry.meta.title, entry.meta.deleted_at
        )));
        title.set_xalign(0.0);
        title.set_hexpand(true);
        row.append(&title);
        let restore = gtk::Button::with_label("Restaurar");
        let app = app.clone();
        let window = window.clone();
        let path = entry.dir.clone();
        restore.connect_clicked(move |_| match app.state.borrow_mut().restore_trash(&path) {
            Ok(_) => {
                app.refresh_all();
                app.toast("Restaurado");
                window.close();
            }
            Err(error) => app.report(&error),
        });
        row.append(&restore);
        list.append(&row);
    }
    let purge = gtk::Button::with_label("Vaciar entradas de más de 30 días");
    let app_purge = app.clone();
    purge.connect_clicked(move |_| {
        let state = app_purge.state.borrow();
        match folio_store::trash::purge(&state.store.vault, 30, &state.clock, state.store.crypto())
        {
            Ok(count) => {
                drop(state);
                app_purge.toast(&format!("{count} entradas purgadas"));
            }
            Err(error) => {
                drop(state);
                app_purge.report(&error);
            }
        }
    });
    box_.append(&purge);
    window.set_child(Some(&box_));
    window.present();
}

pub fn prompt_unlock(app: &Rc<App>) {
    let window = gtk::Window::builder()
        .title("Desbloquear bóveda")
        .default_width(460)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 10);
    box_.set_margin_top(18);
    box_.set_margin_bottom(18);
    box_.set_margin_start(18);
    box_.set_margin_end(18);
    let title = gtk::Label::new(Some("Bóveda bloqueada"));
    title.add_css_class("empty-title");
    box_.append(&title);
    let hint = gtk::Label::new(Some(
        "Introduce la contraseña o la frase de recuperación de 24 palabras.",
    ));
    hint.add_css_class("caption");
    hint.set_wrap(true);
    box_.append(&hint);
    let password = gtk::PasswordEntry::new();
    password.set_placeholder_text(Some("Contraseña"));
    password.set_show_peek_icon(true);
    password.set_activates_default(true);
    box_.append(&password);
    let recovery = gtk::Entry::new();
    recovery.set_placeholder_text(Some("Frase de recuperación (opcional)"));
    box_.append(&recovery);
    let error = gtk::Label::new(None);
    error.add_css_class("missing");
    error.set_wrap(true);
    box_.append(&error);
    let unlock = gtk::Button::with_label("Desbloquear");
    unlock.add_css_class("suggested-action");
    box_.append(&unlock);
    window.set_child(Some(&box_));
    window.present();
    password.grab_focus();

    let app = app.clone();
    let window_for_unlock = window.clone();
    let password_entry = password.clone();
    let recovery_entry = recovery.clone();
    let error_label = error.clone();
    unlock.connect_clicked(move |_| {
        let result = if recovery_entry.text().trim().is_empty() {
            app.unlock_vault(&password_entry.text())
        } else {
            app.unlock_vault_recovery(&recovery_entry.text())
        };
        match result {
            Ok(()) => window_for_unlock.close(),
            Err(error) => error_label.set_text(&error.to_string()),
        }
    });
}

/// Alternativa accesible al lienzo: lista de bloques con acciones de teclado.
pub fn open_block_list(app: &Rc<App>) {
    let blocks: Vec<(String, String, String, bool)> = {
        let state = app.state.borrow();
        let Some(note) = state.current_note.as_ref() else {
            drop(state);
            app.toast("Abre una nota primero");
            return;
        };
        note.blocks
            .iter()
            .map(|block| {
                (
                    block.id.clone(),
                    crate::blocks::kind_label(&block.kind),
                    block
                        .plain_text()
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                    block.placement.is_free(),
                )
            })
            .collect()
    };
    if blocks.is_empty() {
        app.toast("La nota no tiene bloques");
        return;
    }
    let window = gtk::Window::builder()
        .title("Lista de bloques")
        .default_width(640)
        .default_height(520)
        .modal(true)
        .transient_for(&app.window)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);
    let title = gtk::Label::new(Some("Alternativa al lienzo"));
    title.set_xalign(0.0);
    title.add_css_class("pane-title");
    let hint = gtk::Label::new(Some(
        "Recorre los bloques con Tab. Ctrl+Shift+B abre esta lista; Esc vuelve al documento desde el lienzo.",
    ));
    hint.set_wrap(true);
    hint.set_xalign(0.0);
    hint.add_css_class("caption");
    root.append(&title);
    root.append(&hint);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_vexpand(true);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.update_property(&[gtk::accessible::Property::Label("Bloques de la nota")]);
    scroller.set_child(Some(&list));
    root.append(&scroller);
    let close = gtk::Button::with_label("Cerrar");
    close.set_halign(gtk::Align::End);
    root.append(&close);
    window.set_child(Some(&root));

    for (index, (id, kind, preview, free)) in blocks.into_iter().enumerate() {
        let row = gtk::ListBoxRow::new();
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        content.set_margin_top(6);
        content.set_margin_bottom(6);
        content.set_margin_start(8);
        content.set_margin_end(8);
        let text = gtk::Label::new(Some(&format!(
            "{}. {kind}{} — {preview}",
            index + 1,
            if free { " (anclado)" } else { "" }
        )));
        text.set_xalign(0.0);
        text.set_hexpand(true);
        text.set_ellipsize(gtk::pango::EllipsizeMode::End);
        content.append(&text);
        let up = gtk::Button::from_icon_name("go-up-symbolic");
        up.set_tooltip_text(Some("Subir bloque"));
        up.update_property(&[gtk::accessible::Property::Label("Subir bloque")]);
        let down = gtk::Button::from_icon_name("go-down-symbolic");
        down.set_tooltip_text(Some("Bajar bloque"));
        down.update_property(&[gtk::accessible::Property::Label("Bajar bloque")]);
        let anchor = gtk::Button::with_label(if free { "Al flujo" } else { "Anclar" });
        let focus = gtk::Button::with_label("Editar");
        let remove = gtk::Button::from_icon_name("user-trash-symbolic");
        remove.set_tooltip_text(Some("Eliminar bloque"));
        remove.update_property(&[gtk::accessible::Property::Label("Eliminar bloque")]);
        content.append(&up);
        content.append(&down);
        content.append(&anchor);
        content.append(&focus);
        content.append(&remove);
        row.set_child(Some(&content));
        row.update_property(&[gtk::accessible::Property::Label(&format!(
            "Bloque {}: {kind}",
            index + 1
        ))]);

        let app_up = app.clone();
        let window_up = window.clone();
        let id_up = id.clone();
        up.connect_clicked(move |_| {
            app_up.state.borrow_mut().move_block(&id_up, -1);
            app_up.refresh_editor();
            app_up.schedule_save();
            window_up.close();
        });
        let app_down = app.clone();
        let window_down = window.clone();
        let id_down = id.clone();
        down.connect_clicked(move |_| {
            app_down.state.borrow_mut().move_block(&id_down, 1);
            app_down.refresh_editor();
            app_down.schedule_save();
            window_down.close();
        });
        let app_anchor = app.clone();
        let window_anchor = window.clone();
        let id_anchor = id.clone();
        anchor.connect_clicked(move |_| {
            app_anchor
                .state
                .borrow_mut()
                .anchor_block(&id_anchor, !free);
            app_anchor.refresh_editor();
            app_anchor.schedule_save();
            window_anchor.close();
        });
        let app_focus = app.clone();
        let window_focus = window.clone();
        focus.connect_clicked(move |_| {
            {
                let mut state = app_focus.state.borrow_mut();
                state.mode = crate::state::ViewMode::Document;
            }
            app_focus.refresh_editor();
            window_focus.close();
        });
        let app_remove = app.clone();
        let window_remove = window.clone();
        remove.connect_clicked(move |_| {
            app_remove.state.borrow_mut().remove_block(&id);
            app_remove.refresh_editor();
            app_remove.schedule_save();
            window_remove.close();
        });
        list.append(&row);
    }
    let window_close = window.clone();
    close.connect_clicked(move |_| window_close.close());
    window.present();
}
