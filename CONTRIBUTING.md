# Contribuir a Folio

Gracias por querer mejorar Folio. Este proyecto prioriza **integridad de datos**, **rendimiento medido** y **UX nativa**.

## Antes de empezar

1. Lee `AGENTS.md` y `docs/09-hoja-de-ruta.md`.
2. Comprueba la etapa actual en la tabla de estado de `docs/09`.
3. Para cambios de alcance o decisiones, actualiza `docs/10` (ADR) y `docs/09`.
4. No añadas funciones de 1.0 al MVP ni sync/colaboración.

## Entorno

```bash
rustup default stable
rustup component add rustfmt clippy llvm-tools-preview
# Arch
sudo pacman -S --needed base-devel pkgconf gtk4 libadwaita sqlite \
  xorg-server-xvfb dbus
# Debian/Ubuntu
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev \
  libsqlite3-dev xvfb dbus-x11
```

## Flujo

```bash
./scripts/automation/all.sh --fast   # plan + fmt + clippy + tests de dominio
./scripts/automation/all.sh          # añade UI en Xvfb privado
```

- Los tests de UI **nunca** usan tu `DISPLAY`; `all.sh` arranca Xvfb.
- No escribas en `~/Notas` ni `~/.config/folio` desde tests.
- Un bug de datos se convierte en prueba de regresión antes de corregirse.
- Los cambios de configuración deben aparecer en el esquema, la GUI, la CLI y `docs/05`.

## Commits

Formato convencional, en español o inglés:

```text
feat(editor): undo por bloques
fix(store): no perder order_key al mover
docs(plan): actualizar puerta E5
test(store): caos de guardado con kill -9
```

## Estilo

- `cargo fmt --all` y `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Rust estable, edición 2024; sin `unsafe` salvo FFI justificado.
- Errores explícitos; nada de `unwrap()` en rutas de datos.
- Comentarios en español, identificadores en inglés.
- No introduzcas dependencias de GTK en `folio-core`, `folio-store`, `folio-daily`,
  `folio-layout`, `folio-render` ni `folio-plugins`.

## Pull requests

Incluye:

- qué cambia y por qué;
- etapa y puerta afectadas;
- pruebas ejecutadas (pega la salida relevante);
- capturas si cambia la UI (claro y oscuro);
- riesgos y plan de reversión.

## Licencia

Al contribuir aceptas que tu aportación se publique bajo GPL-3.0-or-later para la
aplicación. El SDK (`folio-plugin-sdk`) se distribuye además bajo MIT/Apache-2.0
para facilitar plugins de terceros.
