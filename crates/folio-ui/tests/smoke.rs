//! Smoke test de la ventana GTK. Se ejecuta siempre dentro de Xvfb.
//!
//! GTK solo puede inicializarse en un hilo; por eso todas las comprobaciones
//! viven en una única prueba.

use std::time::{Duration, Instant};

use folio_config::Config;
use folio_core::SystemClock;
use folio_store::Store;
use gtk::prelude::*;
use libadwaita as adw;

#[test]
fn window_opens_document_canvas_and_preview() {
    gtk::init().expect("GTK debe inicializar en el Xvfb privado");
    let dir = tempfile::tempdir().unwrap();
    let clock = SystemClock;
    let store = Store::init(dir.path(), "Smoke", &clock).unwrap();
    let book = store
        .create_book("Smoke", "manual", None, None, "smoke", &clock)
        .unwrap();
    let mut note = store.create_note(&book, "Nota de humo", &clock).unwrap();
    note.blocks.push(folio_core::Block::text_plain("hola"));
    note.blocks.push(folio_core::Block::new(
        "web",
        folio_core::model::BlockContent::Web(folio_core::model::WebContent {
            url: "https://example.com".into(),
            mode: "live".into(),
            title: "Ejemplo".into(),
            ..Default::default()
        }),
    ));
    store.save_note(&book, &mut note, &clock).unwrap();

    let application = adw::Application::builder()
        .application_id("io.github.leonisauro.Folio.Smoke")
        .build();
    let state = folio_ui::state::AppState::new(store, Config::defaults()).unwrap();
    let app = folio_ui::window::App::new(state, &application);
    assert!(app.window.is_visible(), "la ventana debe presentarse");
    app.state.borrow_mut().open_note(&note.id).unwrap();
    app.refresh_all();
    app.refresh_editor();

    let context = glib::MainContext::default();
    let pump = |millis: u64| {
        let deadline = Instant::now() + Duration::from_millis(millis);
        while Instant::now() < deadline {
            while context.iteration(false) {}
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    pump(200);
    assert_eq!(
        app.state.borrow().current_note.as_ref().unwrap().title,
        "Nota de humo"
    );

    app.state.borrow_mut().mode = folio_ui::state::ViewMode::Canvas;
    app.refresh_editor();
    pump(100);
    app.state.borrow_mut().mode = folio_ui::state::ViewMode::Preview;
    app.refresh_editor();
    pump(100);
    // La previsualización se construye en solo lectura.
    app.state.borrow_mut().mode = folio_ui::state::ViewMode::Document;
    app.refresh_editor();
    pump(100);
    app.window.close();
}
