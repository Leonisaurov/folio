# 15 — Automatización y pruebas aisladas en X11

## 1. Objetivo

Que cualquier persona o agente pueda ejecutar la suite completa **sin tocar la sesión gráfica del usuario**, sin ventanas que roben el foco, sin escribir en `~/.config` y sin depender de que el escritorio esté desbloqueado.

La automatización debe ser:

- **Aislada**: Xvfb propio, bus D-Bus propio, `HOME`/XDG temporales.
- **Reproducible**: mismas variables, mismo tamaño de pantalla, mismos artefactos.
- **Rápida**: chequeo del plan en segundos; tests de código por capas.
- **Diagnosticable**: logs y capturas por ejecución.
- **Segura**: sin red por defecto, sin credenciales, sin tocar la bóveda real.

## 2. Regla de oro

> **Los tests de UI nunca usan el `DISPLAY` del usuario.** Siempre arrancan un Xvfb privado con `xvfb-run -a`.

Esto evita:

- ventanas que aparecen encima del trabajo;
- que un test mueva el ratón o el foco;
- que el test lea/escriba la configuración real;
- bloqueos si la sesión está bloqueada o en suspensión.

## 3. Comandos

Desde la raíz del repo:

```bash
# Solo el plan (no requiere Rust ni GTK)
./scripts/automation/plan.sh

# Formato, clippy y auditoría (si hay Cargo.toml)
./scripts/automation/quality.sh

# Tests de dominio/store sin UI
./scripts/automation/unit.sh

# Tests de UI en un Xvfb privado
./scripts/automation/ui.sh

# Suite completa (plan + calidad + unidad + UI)
./scripts/automation/all.sh

# Suite rápida (plan + calidad + unidad, sin UI)
./scripts/automation/all.sh --fast

# Ejecutar un comando arbitrario en el entorno aislado
./scripts/automation/with-x11.sh -- sh -c 'echo "DISPLAY=$DISPLAY"'

# Re-ejecutar al cambiar archivos (requiere inotify-tools)
./scripts/automation/watch.sh

# Dejar la suite corriendo sin que la máquina se suspenda
./scripts/automation/keep-awake.sh -- ./scripts/automation/all.sh

# Dejar el watcher corriendo (la pantalla puede apagarse)
./scripts/automation/keep-awake.sh -- ./scripts/automation/watch.sh

# Suite desatendida con log propio; bloquea suspensión
./scripts/automation/nightly.sh
```

`ui.sh` acepta argumentos de `cargo test`:

```bash
./scripts/automation/ui.sh --test smoke
./scripts/automation/ui.sh --test dnd -- --nocapture
FOLIO_TEST_FEATURES="--features webkit" ./scripts/automation/ui.sh
FOLIO_TEST_A11Y=1 ./scripts/automation/ui.sh --test a11y
```

Por defecto la UI se compila sin features opcionales (por ejemplo, WebKit); los tests que las necesiten las activan explícitamente. `FOLIO_TEST_A11Y=1` deja activo el puente de accesibilidad; sin él se desactiva por velocidad.

## 4. Qué aísla `with-x11.sh`

| Recurso | Aislamiento |
|---|---|
| Servidor X | `xvfb-run -a`, pantalla `1440x900x24`, `-nolisten tcp` |
| Bus de sesión | `dbus-run-session --` |
| `HOME` | Directorio temporal |
| `XDG_CONFIG_HOME` / `CACHE` / `DATA` / `STATE` | Directorios temporales |
| `XDG_RUNTIME_DIR` | Temporal con permisos `0700` |
| `TMPDIR` | Temporal |
| GSettings | `GSETTINGS_BACKEND=memory` |
| Backend gráfico | `GDK_BACKEND=x11`, `WAYLAND_DISPLAY` vacío |
| OpenGL | `LIBGL_ALWAYS_SOFTWARE=1` |
| GStreamer | Registro en el temporal |
| Red | Los tests deben usar mocks; no se configura red real |
| Artefactos | `.artifacts/<fecha>-<pid>/` |

Variables que recibe el test:

- `FOLIO_TEST=1`
- `FOLIO_TEST_ARTIFACTS=<ruta>`
- `FOLIO_TEST_VAULT=<ruta temporal>` (cuando el arnés lo cree)
- `HOME`, `XDG_*`, `TMPDIR` temporales.

Si un test necesita una pantalla más grande o un backend concreto, se añade una opción explícita a `with-x11.sh`; no se reutiliza el display del usuario.

## 5. Estructura

```text
scripts/automation/
├── common.sh       # raíz, logging, comprobaciones, artefactos
├── with-x11.sh     # aislamiento X11 + D-Bus + XDG
├── plan.sh         # valida este plan
├── quality.sh      # fmt + clippy + deny/audit
├── unit.sh         # tests de crates sin GTK
├── ui.sh           # tests de UI dentro de with-x11
├── all.sh          # orquesta todo
├── watch.sh        # re-ejecuta al cambiar
├── keep-awake.sh   # bloquea suspensión; permite apagar pantalla
└── nightly.sh      # suite desatendida con log propio
```

`justfile` expone atajos equivalentes:

```bash
just check       # all.sh
just check-fast  # all.sh --fast
just test-ui     # ui.sh
just test-unit   # unit.sh
just plan        # plan.sh
```

## 6. Artefactos y diagnóstico

Cada ejecución de UI crea:

```text
.artifacts/20261029-101500-12345/
├── command.txt     # comando exacto
├── env.txt         # variables relevantes (sin secretos)
├── command.log     # stdout/stderr
└── screenshots/    # capturas si el test las genera
```

- `.artifacts/` está en `.gitignore`.
- Si un test falla, la ruta se imprime al final.
- `FOLIO_TEST_KEEP=1 ./scripts/automation/ui.sh` conserva además el `HOME` temporal para inspección.
- Los tests deben escribir capturas en `FOLIO_TEST_ARTIFACTS/screenshots/`.
- No se guardan notas reales ni credenciales en artefactos.

### 6.1 Trabajo desatendido: pantalla apagada, máquina despierta

Para dejar una suite larga corriendo y apartarte:

```bash
# Bloquea la suspensión por inactividad, pero permite que la pantalla se apague.
./scripts/automation/keep-awake.sh -- ./scripts/automation/all.sh

# Igual, pero también evita que se suspenda al cerrar la tapa.
# Úsalo solo con el portátil conectado a corriente y ventilado.
./scripts/automation/keep-awake.sh --lid -- ./scripts/automation/watch.sh

# Suite completa desatendida, con log en .artifacts/<run>/nightly.log
./scripts/automation/nightly.sh
```

Cómo funciona:

- `systemd-inhibit --what=sleep --mode=block` impide la suspensión mientras el proceso vive.
- **No** se inhibe `idle`: el salvapantallas y el apagado de pantalla siguen funcionando.
- `--lid` añade `handle-lid-switch`; sin él, cerrar la tapa puede suspender el equipo.
- El bloqueo desaparece al terminar el comando; no cambia la configuración de energía del sistema.
- Comprobación: `systemd-inhibit --list | grep -i folio`.
- Si no hay `systemd-inhibit` (contenedor, sistema sin logind), el script avisa y ejecuta el comando igualmente.

Buenas prácticas:

- Conectar el portátil a corriente antes de una ejecución larga.
- No cerrar la tapa sin `--lid`; con `--lid`, vigilar temperatura y ventilación.
- No dejar suites de benchmark/estrés sin supervisión durante horas.
- La pantalla puede apagarse; el test no depende de que esté encendida.
- Si la sesión se bloquea, los tests siguen; no necesitan interacción.

## 7. Cómo escribir tests de UI

1. **Un solo hilo**: GTK no se lleva bien con tests paralelos; `ui.sh` añade `--test-threads=1` por defecto.
2. **Esperas deterministas**: nada de `sleep(1)`. Usar `glib::timeout_future`, señales o condiciones con timeout.
3. **Reloj inyectable**: los tests de libros diarios usan `Clock` falso, no la hora real.
4. **Bóveda temporal**: `tempfile::tempdir()` o `FOLIO_TEST_VAULT`; nunca `~/Notas`.
5. **Sin red**: mockear metadatos web; si un test necesita red, se marca como `#[ignore]` y se ejecuta aparte.
6. **Capturas**: guardar en `FOLIO_TEST_ARTIFACTS/screenshots/<test>.png`; comparación perceptual con umbral, no hash exacto.
7. **Accesibilidad**: usar el árbol accesible de GTK; `NO_AT_BRIDGE=1` por defecto por velocidad, pero los tests de a11y lo desactivan.
8. **Limpieza**: cerrar ventanas y liberar widgets; el test no debe dejar procesos GStreamer/WebKit.

## 8. CI

`.github/workflows/ci.yml`:

- Ejecuta `scripts/automation/plan.sh` siempre.
- Si existe `Cargo.toml`, instala Rust/GTK y ejecuta `scripts/automation/all.sh`.
- Instala `xvfb`, `dbus-x11` y utilidades de GTK.
- Guarda `.artifacts/` como artefacto del workflow si falla.
- No usa el display del runner para tests de UI: `all.sh` ya arranca Xvfb.

## 9. Límites

- **Xvfb es X11, no Wayland.** Los tests cubren la lógica y la UI, pero no las rutas específicas del compositor Wayland. Se mantiene un smoke manual en GNOME/Wayland antes de cada release.
- **Rendimiento**: los benchmarks de FPS y latencia se miden en la sesión real, no en Xvfb (el renderizado por software distorsiona).
- **WebKit/GStreamer**: los tests que los usan pueden necesitar features y paquetes extra; el pipeline base debe seguir funcionando sin ellos.
- **Paralelismo**: un Xvfb por ejecución; si se paraleliza, cada worker debe tener su propio display y su propio `HOME` temporal. No compartir `XDG_RUNTIME_DIR`.

## 10. Criterios de aceptación

- [ ] `./scripts/automation/plan.sh` pasa sin Rust instalado.
- [ ] `./scripts/automation/all.sh` pasa en una máquina sin `Cargo.toml` (solo plan).
- [ ] `./scripts/automation/with-x11.sh -- sh -c 'echo "DISPLAY=$DISPLAY"'` muestra un display distinto de `:0`.
- [ ] Los tests de UI no crean ventanas en la sesión del usuario.
- [ ] Al fallar, se imprime la ruta de artefactos.
- [ ] `FOLIO_TEST_KEEP=1` conserva el entorno temporal.
- [ ] CI ejecuta la misma suite que local.
- [ ] `keep-awake.sh` bloquea la suspensión y permite apagar la pantalla.
- [ ] `nightly.sh` deja log en `.artifacts/` y termina con el código de la suite.
- [ ] Un test que use `sleep` o `~/Notas` se rechaza en revisión.


### 9.1 WebKit en CI

GitHub Actions bloquea `bwrap`/user namespaces. Por eso el CI solo **compila y
enlaza** el test de WebKit; el aislamiento real (perfil efímero, cookies, storage,
WebGL y permisos) se ejecuta en local con `scripts/webkit.sh`. No se desactiva el
sandbox en la app distribuida.
