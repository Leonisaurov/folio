//! Auditoría automatizada básica de accesibilidad.
//!
//! Se ejecuta con `scripts/a11y.sh`, que fuerza escala 200 % y el puente de
//! accesibilidad dentro de Xvfb. No sustituye a una revisión con lector de
//! pantalla, pero cubre foco, etiquetas, tema y alternativa al lienzo.

use std::time::{Duration, Instant};

use folio_config::Config;
use folio_core::{Block, FixedClock, SystemClock};
use folio_store::Store;
use gtk::prelude::*;
use libadwaita as adw;

fn pump(millis: u64) {
    let context = glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_millis(millis);
    while Instant::now() < deadline {
        while context.iteration(false) {}
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn accessible_focus_labels_theme_and_canvas_alternative() {
    gtk::init().expect("GTK en Xvfb");
    let dir = tempfile::tempdir().unwrap();
    let clock = SystemClock;
    let store = Store::init(dir.path(), "A11y", &clock).unwrap();
    let book = store
        .create_book("Accesible", "manual", None, None, "a11y", &clock)
        .unwrap();
    let mut note = store.create_note(&book, "Nota accesible", &clock).unwrap();
    note.blocks.push(Block::text_plain("Texto de la nota"));
    let mut free = Block::text_plain("Bloque anclado");
    free.placement = folio_core::Placement::free(100.0, 100.0, 200.0, 80.0);
    note.blocks.push(free);
    store.save_note(&book, &mut note, &clock).unwrap();

    let application = adw::Application::builder()
        .application_id("io.github.leonisauro.Folio.A11y")
        .build();
    let state = folio_ui::state::AppState::new(store, Config::defaults()).unwrap();
    let app = folio_ui::window::App::new(state, &application);
    app.state.borrow_mut().open_note(&note.id).unwrap();
    app.refresh_all();
    pump(100);

    // Foco visible en el título (Xvfb no tiene gestor de ventanas, así que se
    // comprueba el foco lógico de la ventana, no el foco del servidor X).
    app.window.present();
    pump(100);
    gtk::prelude::RootExt::set_focus(&app.window, Some(&app.title_entry));
    pump(50);
    assert!(gtk::prelude::WidgetExt::is_visible(&app.title_entry));
    assert!(gtk::prelude::WidgetExt::is_sensitive(&app.title_entry));
    assert!(
        gtk::prelude::RootExt::focus(&app.window).is_some(),
        "la ventana debe tener foco lógico"
    );

    // Etiquetas accesibles en botones de icono.
    fn walk(widget: &gtk::Widget, buttons: &mut Vec<gtk::Button>) {
        if let Some(button) = widget.downcast_ref::<gtk::Button>() {
            buttons.push(button.clone());
        }
        let mut child = widget.first_child();
        while let Some(current) = child {
            walk(&current, buttons);
            child = current.next_sibling();
        }
    }
    let mut buttons = Vec::new();
    walk(app.window.upcast_ref(), &mut buttons);
    assert!(!buttons.is_empty());
    for button in buttons {
        if button.icon_name().is_some() {
            assert!(
                button.tooltip_text().is_some(),
                "botón de icono sin descripción: {:?}",
                button.icon_name()
            );
        }
    }

    // Tema claro/oscuro en vivo.
    let manager = adw::StyleManager::default();
    let mut dark = Config::defaults();
    dark.set_memory("ui.theme", serde_json::json!("dark"));
    folio_ui::window::apply_theme(&dark);
    assert_eq!(manager.color_scheme(), adw::ColorScheme::ForceDark);
    let mut light = Config::defaults();
    light.set_memory("ui.theme", serde_json::json!("light"));
    folio_ui::window::apply_theme(&light);
    assert_eq!(manager.color_scheme(), adw::ColorScheme::ForceLight);

    // Alternativa al lienzo: debe abrir una ventana con la lista de bloques.
    let before = gtk::Window::list_toplevels().len();
    folio_ui::dialogs::open_block_list(&app);
    pump(100);
    let after = gtk::Window::list_toplevels().len();
    assert!(after > before, "la lista de bloques debe abrirse");
    for window in gtk::Window::list_toplevels() {
        if window != app.window.clone().upcast::<gtk::Widget>()
            && let Some(window) = window.downcast_ref::<gtk::Window>()
        {
            window.close();
        }
    }

    // Escala configurada por el arnés (200 %).
    if let Ok(scale) = std::env::var("GDK_DPI_SCALE") {
        assert_eq!(scale, "2", "el arnés de a11y debe usar escala 200 %");
    }
    app.window.close();
    let _ = FixedClock::at("2026-10-29T09:00:00Z");
}
