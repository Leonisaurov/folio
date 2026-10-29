//! Vista web viva con WebKitGTK 6.0, aislada y efímera.
//!
//! Solo se compila con `--features webkit`. Cada vista usa una
//! `NetworkSession` efímera: sin cookies persistentes, sin almacenamiento
//! local, sin caché de página, sin WebGL, sin descargas y con todos los
//! permisos denegados. Al desmontarse el widget se termina el proceso web.

use std::rc::Rc;

use folio_core::model::WebContent;
use gtk::prelude::*;
use webkit6::prelude::*;

use crate::window::App;

pub struct EphemeralWeb {
    pub view: webkit6::WebView,
    pub session: webkit6::NetworkSession,
}

/// Crea la vista con las garantías de aislamiento del MVP de 1.0.
pub fn create_ephemeral() -> EphemeralWeb {
    let session = webkit6::NetworkSession::new_ephemeral();
    if let Some(cookies) = session.cookie_manager() {
        cookies.set_accept_policy(webkit6::CookieAcceptPolicy::Never);
    }
    // Cualquier descarga se cancela en el acto.
    session.connect_download_started(|_, download| download.cancel());

    let view = webkit6::WebView::builder()
        .network_session(&session)
        .build();
    if let Some(settings) = webkit6::prelude::WebViewExt::settings(&view) {
        settings.set_enable_webgl(false);
        settings.set_enable_html5_local_storage(false);
        settings.set_enable_html5_database(false);
        settings.set_enable_offline_web_application_cache(false);
        settings.set_enable_page_cache(false);
        settings.set_enable_developer_extras(false);
        // Sin acceso a archivos locales desde el contenido remoto.
        settings.set_allow_file_access_from_file_urls(false);
        settings.set_allow_universal_access_from_file_urls(false);
    }
    // Permisos (cámara, micrófono, geolocalización, portapapeles…): denegados.
    view.connect_permission_request(|_, _| false);
    // Las ventanas emergentes no se abren dentro de Folio.
    view.connect_create(|_, _| None::<gtk::Widget>);
    view.set_background_color(&gtk::gdk::RGBA::WHITE);

    EphemeralWeb { view, session }
}

/// Construye el bloque con tarjeta de estado y confirmación explícita.
pub fn render(app: &Rc<App>, block: &folio_core::Block, web: &WebContent) -> gtk::Widget {
    let wrapper = gtk::Box::new(gtk::Orientation::Vertical, 6);
    wrapper.add_css_class("web-live");

    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let badge = gtk::Label::new(Some("Vista viva efímera · sin cookies · sin descargas"));
    badge.add_css_class("caption");
    badge.set_hexpand(true);
    badge.set_xalign(0.0);
    toolbar.append(&badge);

    let url = gtk::Label::new(Some(&web.url));
    url.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    url.set_xalign(0.0);
    url.add_css_class("caption");
    toolbar.append(&url);
    wrapper.append(&toolbar);

    let host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    host.set_size_request(-1, 420);
    wrapper.append(&host);

    let placeholder = gtk::Label::new(Some(
        "La vista viva no se carga hasta que lo confirmes.\nSe descarta al salir de la nota y no guarda cookies.",
    ));
    placeholder.set_justify(gtk::Justification::Center);
    placeholder.add_css_class("caption");
    placeholder.set_vexpand(true);
    host.append(&placeholder);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let load = gtk::Button::with_label("Cargar vista viva");
    load.add_css_class("suggested-action");
    let open_external = gtk::Button::with_label("Abrir en navegador");
    buttons.append(&load);
    buttons.append(&open_external);
    wrapper.append(&buttons);

    {
        let app = app.clone();
        let web_url = web.url.clone();
        open_external.connect_clicked(move |_| {
            let dialog = gtk::AlertDialog::builder()
                .message(format!("¿Abrir {web_url} en el navegador del sistema?"))
                .buttons(["Cancelar", "Abrir"])
                .cancel_button(0)
                .default_button(0)
                .build();
            let web_url = web_url.clone();
            dialog.choose(
                Some(&app.window),
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
    }

    {
        let app = app.clone();
        let web = web.clone();
        let block_id = block.id.clone();
        let host = host.clone();
        let load = load.clone();
        let load_button = load.clone();
        load.connect_clicked(move |_| {
            if folio_core::validate::validate_url(&web.url).is_err() {
                app.toast("URL no permitida para vista viva");
                return;
            }
            let dialog = gtk::AlertDialog::builder()
                .message("¿Cargar la vista web viva?")
                .detail("Se usará un perfil efímero sin cookies persistentes, sin descargas y con permisos denegados. El contenido remoto es no confiable.")
                .buttons(["Cancelar", "Cargar"])
                .cancel_button(0)
                .default_button(0)
                .build();
            let app_load = app.clone();
            let web = web.clone();
            let block_id = block_id.clone();
            let host = host.clone();
            let load_button = load_button.clone();
            dialog.choose(Some(&app.window), None::<&gio::Cancellable>, move |result| {
                if result != Ok(1) {
                    return;
                }
                let ephemeral = create_ephemeral();
                let view = ephemeral.view.clone();
                let session = ephemeral.session.clone();
                let app_close = app_load.clone();
                let host_close = host.clone();
                let block_id_close = block_id.clone();
                // Mientras el widget viva, la sesión efímera también.
                view.connect_unrealize(move |view| {
                    let _ = &session;
                    view.stop_loading();
                    view.terminate_web_process();
                });
                view.connect_load_failed(|_view, _event, uri, error| {
                    tracing::warn!("vista web falló {uri}: {error}");
                    false
                });
                view.load_uri(&web.url);
                while let Some(child) = host.first_child() {
                    host.remove(&child);
                }
                let status = gtk::Label::new(Some(&format!("Cargando {}…", web.url)));
                status.add_css_class("caption");
                host.append(&status);
                host.append(&view);
                load_button.set_sensitive(false);

                let close = gtk::Button::with_label("Descargar contenido y cerrar");
                let app_for_close = app_close.clone();
                let host_for_close = host_close.clone();
                let block_id_for_close = block_id_close.clone();
                let view_for_close = view.clone();
                close.connect_clicked(move |_| {
                    view_for_close.stop_loading();
                    view_for_close.terminate_web_process();
                    while let Some(child) = host_for_close.first_child() {
                        host_for_close.remove(&child);
                    }
                    // Volver a tarjeta: la próxima apertura vuelve a pedir
                    // confirmación y crea otra sesión efímera.
                    if let Some(note) = app_for_close.state.borrow_mut().current_note.as_mut()
                        && let Some(block) = note.block_mut(&block_id_for_close)
                        && let folio_core::model::BlockContent::Web(w) = &mut block.content
                    {
                        w.mode = "card".into();
                    }
                    app_for_close.refresh_editor();
                    app_for_close.schedule_save();
                });
                host.append(&close);
            });
        });
    }
    wrapper.upcast()
}
