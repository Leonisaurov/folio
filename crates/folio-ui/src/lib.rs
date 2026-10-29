//! Interfaz GTK4/Libadwaita de Folio.

pub mod blocks;
pub mod dialogs;
pub mod helpers;
#[cfg(feature = "webkit")]
pub mod live_web;
pub mod state;
pub mod window;

use std::path::PathBuf;
use std::rc::Rc;

use folio_config::Config;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::state::AppState;
use crate::window::App;

const APP_ID: &str = "io.github.leonisauro.Folio";

/// Arranca la aplicación gráfica y no retorna hasta que se cierra.
pub fn run(vault: Option<PathBuf>, config: Config, quit_after_init: bool) -> glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();

    let config_for_activate = config.clone();
    let vault_for_activate = vault.clone();
    let config_startup = config.clone();
    application.connect_startup(move |application| {
        load_css();
        crate::window::apply_theme(&config_startup);
        application.set_accels_for_action("app.quit", &["<Control>q"]);
    });

    application.connect_activate(move |application| {
        match &vault_for_activate {
            Some(path) => match open_vault(application, path, &config_for_activate) {
                Ok(_) => {}
                Err(error) => show_fatal(application, &error.to_string()),
            },
            None => show_welcome(application, &config_for_activate),
        }
        if quit_after_init {
            let application = application.clone();
            // Un idle basta para medir «hasta ventana interactiva» sin añadir
            // una espera artificial al benchmark.
            glib::idle_add_local_once(move || application.quit());
        }
    });

    // Los flags ya los consumió clap; GApplication no debe reinterpretarlos.
    application.run_with_args(&["folio"])
}

pub fn open_vault(
    application: &adw::Application,
    path: &std::path::Path,
    config: &Config,
) -> Result<Rc<App>, folio_core::FolioError> {
    let store = folio_store::Store::open(path)?;
    let config = match crate::helpers::load_config(Some(path)) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("configuración inválida: {error}");
            config.clone()
        }
    };
    let state = AppState::new(store, config)?;
    Ok(App::new(state, application))
}

fn show_welcome(application: &adw::Application, config: &Config) {
    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Folio")
        .default_width(620)
        .default_height(440)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 16);
    box_.set_valign(gtk::Align::Center);
    box_.set_halign(gtk::Align::Center);
    box_.set_margin_start(36);
    box_.set_margin_end(36);
    let title = gtk::Label::new(Some("Bienvenido a Folio"));
    title.add_css_class("welcome-title");
    box_.append(&title);
    let subtitle = gtk::Label::new(Some(
        "Notas y libros diarios en archivos legibles. Elige una bóveda para empezar.\nNo se sube nada a la red.",
    ));
    subtitle.set_justify(gtk::Justification::Center);
    subtitle.add_css_class("caption");
    box_.append(&subtitle);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::Center);
    let create = gtk::Button::with_label("Crear bóveda");
    create.add_css_class("suggested-action");
    let open = gtk::Button::with_label("Abrir bóveda");
    buttons.append(&create);
    buttons.append(&open);
    box_.append(&buttons);
    let hint = gtk::Label::new(Some(
        "También puedes lanzar `folio --vault <ruta>` o `folio vault init <ruta>`.",
    ));
    hint.add_css_class("caption");
    box_.append(&hint);
    toolbar.set_content(Some(&box_));
    window.set_content(Some(&toolbar));
    window.present();

    let window_create = window.clone();
    let application_create = application.clone();
    let config_create = config.clone();
    create.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title("Elegir carpeta para la nueva bóveda")
            .build();
        let window = window_create.clone();
        let application = application_create.clone();
        let config = config_create.clone();
        dialog.select_folder(
            Some(&window_create),
            None::<&gio::Cancellable>,
            move |result| {
                let Ok(file) = result else { return };
                let Some(path) = file.path() else { return };
                let clock = folio_core::SystemClock;
                match folio_store::Store::init(&path, "Folio", &clock) {
                    Ok(_) => {
                        let _ = crate::helpers::remember_vault(&path);
                        match open_vault(&application, &path, &config) {
                            Ok(_) => window.close(),
                            Err(error) => show_fatal(&application, &error.to_string()),
                        }
                    }
                    Err(error) => show_fatal(&application, &error.to_string()),
                }
            },
        );
    });

    let window_open = window.clone();
    let application_open = application.clone();
    let config_open = config.clone();
    open.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::builder()
            .title("Abrir bóveda existente")
            .build();
        let window = window_open.clone();
        let application = application_open.clone();
        let config = config_open.clone();
        dialog.select_folder(
            Some(&window_open),
            None::<&gio::Cancellable>,
            move |result| {
                let Ok(file) = result else { return };
                let Some(path) = file.path() else { return };
                match open_vault(&application, &path, &config) {
                    Ok(_) => {
                        let _ = crate::helpers::remember_vault(&path);
                        window.close();
                    }
                    Err(error) => show_fatal(&application, &error.to_string()),
                }
            },
        );
    });
}

fn show_fatal(application: &adw::Application, message: &str) {
    let dialog = gtk::AlertDialog::builder()
        .message("No se pudo abrir Folio")
        .detail(message)
        .build();
    dialog.show(application.active_window().as_ref());
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
