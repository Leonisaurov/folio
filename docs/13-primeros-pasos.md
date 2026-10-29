# 13 — Primeros pasos: del plan al código

**Objetivo de esta guía**: que al final del día 1 exista un repositorio compilable, con CI y una ventana GTK, sin haber escrito todavía la lógica de notas.

## 1. Antes de tocar código

### 1.1 Confirmar cinco cosas

- [ ] Nombre definitivo (o aceptar «Folio» provisional).
- [ ] Licencia (propuesta: GPL-3.0-or-later para la app; MIT/Apache-2.0 para el SDK).
- [ ] Repositorio remoto (GitHub/GitLab) y visibilidad.
- [ ] Distro/runtime de desarrollo (Arch + GNOME/Wayland es el entorno actual).
- [ ] Aceptar que el MVP no incluye sync ni colaboración.

Si alguna cambia, se actualiza la ADR correspondiente antes de empezar, no después.

### 1.2 Dependencias del sistema (Arch)

```bash
sudo pacman -S --needed base-devel rustup git pkgconf \
  gtk4 libadwaita gstreamer gst-plugins-base gst-plugins-good \
  gst-plugins-bad gst-plugins-ugly webkit2gtk-6.0 \
  sqlite flatpak flatpak-builder appstream \
  xorg-server-xvfb dbus inotify-tools
rustup default stable
rustup component add rustfmt clippy llvm-tools-preview
```

> `webkit2gtk-6.0` y `gst-plugins-*` son opcionales para el primer build; se pueden añadir en E7. El núcleo y el editor no los necesitan. `xorg-server-xvfb`/`dbus` son para automatización aislada; `inotify-tools` es opcional para `watch.sh`.

### 1.3 Herramientas de desarrollo

```bash
cargo install just cargo-deny cargo-audit cargo-llvm-cov hyperfine
cargo install --locked wasm-tools
```

`wasm-tools` se usa en E10; no es necesario el día 1.

## 2. Crear el repositorio

```bash
mkdir -p ~/Develop/C/Notas && cd ~/Develop/C/Notas
git init
git branch -M main

# Estructura mínima
mkdir -p crates/folio-core/src crates/folio-store/src crates/folio-config/src \
         crates/folio-daily/src crates/folio-layout/src crates/folio-render/src \
         crates/folio-plugins/src crates/folio-ui/src crates/folio-app/src \
         crates/folio-plugin-sdk/src plugins docs scripts flatpak tests/fixtures
```

### 2.1 `Cargo.toml` del workspace

```toml
[workspace]
resolver = "2"
members = [
  "crates/folio-core",
  "crates/folio-store",
  "crates/folio-config",
  "crates/folio-daily",
  "crates/folio-layout",
  "crates/folio-render",
  "crates/folio-plugins",
  "crates/folio-ui",
  "crates/folio-app",
  "crates/folio-plugin-sdk",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "GPL-3.0-or-later"
rust-version = "1.92"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
uuid = { version = "1", features = ["v7", "serde"] }
jiff = "0.2"
rusqlite = { version = "0.32", features = ["bundled", "backup"] }
toml = "0.8"
toml_edit = "0.22"
clap = { version = "4", features = ["derive", "env"] }
tokio = { version = "1", features = ["rt-multi-thread", "sync", "time", "fs", "macros"] }
async-channel = "2"
notify = "8"
blake3 = "1"
```

> Las versiones son orientativas; se fijan con `cargo update` y `Cargo.lock`. No copiar a ciegas: validar compatibilidad con GTK 4.14+.

### 2.2 Perfiles

```toml
[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "unwind"

[profile.dev]
debug = 1
```

### 2.3 `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
```

## 3. Primer crate: `folio-core`

`crates/folio-core/Cargo.toml`:

```toml
[package]
name = "folio-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
uuid.workspace = true
jiff.workspace = true
```

`src/lib.rs`:

```rust
pub mod ids;
pub mod error;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

`src/ids.rs`:

```rust
use uuid::Uuid;

pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}
```

`src/error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum FolioError {
    #[error("no encontrado: {0}")]
    NotFound(String),
    #[error("datos inválidos: {0}")]
    Invalid(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
```

## 4. Primer binario: ventana GTK

`crates/folio-app/Cargo.toml`:

```toml
[package]
name = "folio-app"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "folio"
path = "src/main.rs"

[dependencies]
folio-core = { path = "../folio-core" }
gtk = { package = "gtk4", version = "0.11", features = ["v4_14"] }
libadwaita = { version = "0.9", features = ["v1_5"] }
```

`src/main.rs`:

```rust
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("io.github.leonisauro.Folio")
        .build();

    app.connect_activate(|app| {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Folio")
            .default_width(1100)
            .default_height(760)
            .build();

        let header = adw::HeaderBar::new();
        let label = gtk::Label::new(Some("Folio · plan en construcción"));
        label.set_margin_top(24);
        label.set_margin_bottom(24);

        let view = adw::ToolbarView::new();
        view.add_top_bar(&header);
        view.set_content(Some(&label));
        window.set_content(Some(&view));
        window.present();
    });

    app.run()
}
```

> Ajustar versiones de crates a las disponibles; el objetivo es una ventana, no la UI final.

## 5. Calidad y CI

### 5.1 `justfile` y scripts

El repo ya incluye `justfile` y `scripts/automation/`:

```bash
just plan        # solo valida el plan
just check-fast  # plan + calidad + unidad (sin UI)
just check       # suite completa, UI en Xvfb privado
just test-ui     # solo UI aislada
just test-watch  # re-ejecuta al cambiar
```

Sin `just`:

```bash
./scripts/automation/plan.sh
./scripts/automation/all.sh --fast
./scripts/automation/all.sh
```

La UI nunca usa tu `DISPLAY`: `with-x11.sh` arranca Xvfb, D-Bus y `HOME`/XDG temporales. Para dejar una suite larga sin que el equipo se suspenda:

```bash
./scripts/automation/keep-awake.sh -- ./scripts/automation/all.sh
./scripts/automation/nightly.sh
```

La pantalla puede apagarse; la suspensión queda bloqueada. Detalle: [15-automatizacion.md](15-automatizacion.md).

### 5.2 `.github/workflows/ci.yml`

Ya existe una plantilla en `.github/workflows/ci.yml`:

- valida el plan siempre;
- si hay `Cargo.toml`, instala Rust/GTK y ejecuta `scripts/automation/all.sh`;
- sube `.artifacts/` si falla;
- no usa el display del runner.

> Ajustar paquetes según la imagen; si WebKit 6.0 no está, compilar sin la feature `webkit` y añadir un job específico en Flatpak.

### 5.3 `deny.toml`

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "ISC", "MPL-2.0", "LGPL-3.0-or-later", "GPL-3.0-or-later"]

[bans]
multiple-versions = "warn"
```

## 6. Primer commit

```bash
git add .
git commit -m "chore: esqueleto del proyecto, ventana GTK y CI"
git remote add origin <url>
git push -u origin main
```

## 7. Checklist del día 1

- [ ] Repo creado y remoto configurado.
- [ ] `cargo build --workspace` pasa.
- [ ] `./scripts/automation/all.sh` pasa (la UI se ejecuta en Xvfb, no en tu sesión).
- [ ] `./scripts/automation/with-x11.sh -- sh -c 'echo $DISPLAY'` muestra un display distinto de `:0`.
- [ ] La ventana GTK abre en Wayland (smoke manual).
- [ ] CI verde.
- [ ] `just check` pasa.
- [ ] `./scripts/automation/keep-awake.sh -- true` pasa y la pantalla puede apagarse.
- [ ] `docs/10-decisiones.md` con ADR-001..003.
- [ ] `baseline.json` con el primer benchmark de arranque.
- [ ] Issues iniciales creadas para E0/E1.

## 8. Checklist de la semana 1

- [ ] E0 completo.
- [ ] E1: modelo, bóveda, escritura atómica y tests de roundtrip.
- [ ] Primer `folio vault init`.
- [ ] Primer `folio note create`.
- [ ] Prueba `kill -9` inicial.
- [ ] Demo del viernes: crear bóveda y nota desde CLI; ventana que la muestra (aunque sea en crudo).

## 9. Lo que NO se debe hacer todavía

- No diseñar la UI final antes de tener el modelo estable.
- No implementar plugins antes de E10.
- No añadir sync.
- No optimizar prematuramente el editor.
- No elegir un ORM: SQLite directo.
- No crear un sistema de migraciones casero sin tests.
- No prometer fechas públicas antes de la beta.
- No escribir el editor propio desde cero (spike primero).
- No usar `unwrap()` en rutas de datos; errores explícitos.
- No guardar secretos en TOML.

## 10. Cómo retomar cada mañana

1. Leer la etapa actual en [09-hoja-de-ruta.md](09-hoja-de-ruta.md).
2. Elegir una tarea de ≤ 1 día.
3. Escribir la prueba que falla (si es un bug o un invariante).
4. Implementar.
5. Ejecutar `./scripts/automation/all.sh --fast` mientras iteras y `./scripts/automation/all.sh` antes de cerrar.
6. Actualizar el estado de la etapa y las ADR si cambió algo.
7. Commit pequeño con mensaje convencional.

## 11. Criterio para pedir ayuda

Si estás bloqueado más de 2 horas en:

- compilación de GTK/WebKit/GStreamer,
- comportamiento de IME o accesibilidad,
- corrupción de datos,
- sandbox de plugins,

para y documenta el bloqueo con un caso mínimo reproducible. No sigas construyendo sobre una base inestable: el objetivo del plan es precisamente evitar eso.
