//! Pruebas de aislamiento de la vista web viva (solo con `--features webkit`).
#![cfg(feature = "webkit")]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use webkit6::prelude::*;

#[test]
fn ephemeral_webview_has_no_persistent_state_and_unloads() {
    gtk::init().expect("GTK debe inicializar en Xvfb");
    let ephemeral = folio_ui::live_web::create_ephemeral();
    let view = ephemeral.view.clone();
    let session = ephemeral.session.clone();

    assert!(session.is_ephemeral(), "la NetworkSession debe ser efímera");
    let cookies = session.cookie_manager().expect("cookie manager");
    cookies.set_accept_policy(webkit6::CookieAcceptPolicy::Never);

    let settings = webkit6::prelude::WebViewExt::settings(&view).expect("settings");
    assert!(!settings.enables_webgl());
    assert!(!settings.enables_html5_local_storage());
    assert!(!settings.enables_html5_database());
    assert!(!settings.enables_page_cache());
    assert!(!settings.enables_developer_extras());
    assert!(!settings.allows_file_access_from_file_urls());
    assert!(!settings.allows_universal_access_from_file_urls());

    let network = view.network_session().expect("network session");
    assert!(network.is_ephemeral());

    let finished = Rc::new(Cell::new(false));
    let finished_signal = finished.clone();
    let loop_ = glib::MainLoop::new(None, false);
    let loop_signal = loop_.clone();
    view.connect_load_changed(move |_, event| {
        if event == webkit6::LoadEvent::Finished {
            finished_signal.set(true);
            loop_signal.quit();
        }
    });
    let loop_timeout = loop_.clone();
    glib::timeout_add_local_once(Duration::from_secs(10), move || loop_timeout.quit());
    view.load_html(
        "<!doctype html><meta charset=utf-8><h1>Folio</h1><script>try{localStorage.setItem('x','1')}catch(e){}</script>",
        None,
    );
    loop_.run();
    assert!(finished.get(), "la vista web no terminó de cargar en 10 s");

    // Simula salir de la nota: el proceso web se termina explícitamente.
    view.stop_loading();
    view.terminate_web_process();
    assert!(session.is_ephemeral());
}
