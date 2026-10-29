# 14 — Matriz de trazabilidad

Cada requisito del producto tiene una etapa que lo construye y una prueba que lo verifica. Si un requisito no aparece aquí, no está en el alcance; si una etapa no tiene requisito, sobra.

## 1. Requisitos funcionales

| ID | Requisito | Etapa | Verificación |
|---|---|---|---|
| RF-01 | Crear/abrir bóveda local | E1 | `vault init`, roundtrip, `doctor` |
| RF-02 | Libro diario automático | E3 | Tests DST, zonas, 03:59/04:00 |
| RF-03 | Formato de título/carpeta configurable | E3 | Cambio GUI/TOML en vivo |
| RF-04 | Hora de corte y zona configurables | E3 | `daily.day_start_hour`, `timezone` |
| RF-05 | Creación perezosa y backfill manual | E3 | 10 días → 10 libros, sin duplicados |
| RF-06 | Renombrar libro diario sin duplicar | E3 | Renombrar + reiniciar |
| RF-07 | Libros manuales y renombrables | E2 | CRUD de libros |
| RF-08 | Notas ilimitadas | E1 | 10k notas, benchmark |
| RF-09 | Orden manual intra-libro | E6 | 100 movimientos, invariantes |
| RF-10 | Mover notas entre libros | E6 | DnD + `rename` atómico |
| RF-11 | Spring-load a los 650 ms | E6 | Test E2E con temporizador |
| RF-12 | Selector de posición exacta | E6 | Popover + teclado |
| RF-13 | Alternativa por teclado para mover | E6 | `Ctrl+Shift+M` |
| RF-14 | Órdenes por título/creación/modificación | E2/E6 | No destructivos, persistencia |
| RF-15 | Texto enriquecido | E4 | Golden + IME + undo |
| RF-16 | Listas, checklists, citas, código | E4 | Tests de bloque |
| RF-17 | Tablas editables | E8 | 100×20, CSV |
| RF-18 | Imagen, audio, vídeo, archivo | E7 | Reproducción, límites |
| RF-19 | Web tarjeta | E7 | Metadatos, offline |
| RF-20 | Web viva opcional y aislada | E7 | Sin cookies/descargas/red por defecto |
| RF-21 | Dibujo con presión | E8 | Stylus, SVG/PNG |
| RF-22 | Formato y previsualización | E4/E9 | Solo lectura, medios |
| RF-23 | Lienzo infinito con flujo por defecto | E5 | 500 bloques, hit-testing |
| RF-24 | Anclar/desanclar bloques | E5 | Drag, undo, persistencia |
| RF-25 | Recomponer al flujo | E5 | Orden visual, undo |
| RF-26 | Zoom, paneo, encuadres | E5 | 25–400 %, Ctrl+0/1/2 |
| RF-27 | Búsqueda global | E9 | < 80 ms en 10k |
| RF-28 | Filtros por libro/fecha/tipo | E9 | Resultados correctos |
| RF-29 | Exportar Markdown/HTML/PDF | E9 + 1.0 | Golden MD/HTML; PDF en 1.0 |
| RF-30 | Importar Markdown | E9 | Nota válida + assets |
| RF-31 | Plugins WASM | E10 + 1.0 | 3 ejemplos MVP; 5 en 1.0; sandbox |
| RF-32 | Plugin de barra/progreso | E10 + 1.0 | `progress` en MVP; `quickbar` en 1.0 |
| RF-33 | Plugin de gráficas | E10 | `chart` lee tabla |
| RF-34 | Plugin de diagramas | E10 | `diagram` DSL interactivo |
| RF-35 | Gestor de plugins y permisos | E10 | Instalar/revocar/desinstalar |
| RF-36 | Config por texto, GUI y CLI | E3/E5 | Paridad de claves |
| RF-37 | Recarga en vivo y errores TOML | E3 | Última config válida |
| RF-38 | Tema claro/oscuro y tipografía | E2/E12 | Cambio en vivo |
| RF-39 | Papelera y restauración | E1/E11 | Restaurar nota/libro |
| RF-40 | Historial de versiones | E11 | Restaurar versión |
| RF-41 | Backups y verificación | E11 | Hash idéntico |
| RF-42 | Cifrado opcional | 1.0 | `strings`, auto-lock |
| RF-43 | Recuperación tras crash | E1/E4/E11 | `kill -9` |
| RF-44 | Instancia única y deep links | E2 | Segundo launch, `folio://` |
| RF-45 | CLI completa | E1–E11 | `--help`, subcomandos |
| RF-46 | Onboarding | E12 | Primera ejecución < 60 s |
| RF-47 | Accesibilidad | E4/E5/E12 | Teclado, lector, 200 % |
| RF-48 | i18n español | E2/E12 | Sin strings hardcodeados |
| RF-49 | Empaquetado Flatpak/AUR/deb | E13 + 1.0 | Flatpak/AUR en MVP; deb en 1.0 |
| RF-50 | Documentación de usuario y plugin | E13 | Guía probada por tercero |

## 2. Requisitos no funcionales

| ID | Requisito | Etapa | Verificación |
|---|---|---|---|
| RNF-01 | Arranque < 600 ms | E0/E12 | `hyperfine` |
| RNF-02 | RAM < 150 MB con 10k notas | E1/E12 | heaptrack |
| RNF-03 | Tecleo p95 < 16 ms | E4/E12 | instrumentación |
| RNF-04 | Canvas 60 fps con 500 bloques | E5/E12 | frame timing |
| RNF-05 | Búsqueda < 80 ms | E9/E12 | criterion |
| RNF-06 | Guardado < 30 ms | E1/E12 | traza |
| RNF-07 | CPU reposo < 0,5 % | E12 | pidstat/powertop |
| RNF-08 | Binario < 35 MB | E13 | `ls -lh` |
| RNF-09 | Integridad ante crash | E1/E11 | caos |
| RNF-10 | Sandbox de plugins | E10 | pruebas de escape |
| RNF-11 | Sandbox web | E7 | cookies/descargas/red |
| RNF-12 | Cifrado sin texto plano | 1.0 | `strings`, temporales |
| RNF-13 | Accesibilidad por teclado | E4/E5/E12 | checklist |
| RNF-14 | Portabilidad de datos | E1/E9 | abrir sin app, export |
| RNF-15 | Dependencias auditadas | E0/E11 | `cargo deny/audit` |
| RNF-16 | Instalación reproducible | E13 + 1.0 | Flatpak/AUR; deb en 1.0 |
| RNF-17 | Sin telemetría | E0/E12 | revisión de red |
| RNF-18 | i18n preparado | E2/E12 | Fluent |
| RNF-19 | Bajo consumo en reposo | E12 | powertop |
| RNF-20 | Recuperación documentada | E11/E13 | guía + restore |
| RNF-21 | Tests de UI aislados del escritorio | E0/E2 | `automation/ui.sh`; `DISPLAY != :0`; sin ventanas en la sesión |
| RNF-22 | Trabajo desatendido sin suspensión | E0 | `keep-awake.sh`/`nightly.sh`; `systemd-inhibit --list` |

## 3. Requisitos de datos

| ID | Requisito | Etapa | Verificación |
|---|---|---|---|
| RD-01 | JSON versionado | E1 | Roundtrip + migración |
| RD-02 | Campos desconocidos preservados | E1 | Fixture con campo extra |
| RD-03 | Bloque de plugin ausente preservado | E1/E10 | Desinstalar plugin |
| RD-04 | Escritura atómica | E1 | `kill -9` |
| RD-05 | Índice reconstruible | E1 | Borrar `index.sqlite` |
| RD-06 | Orden por nombre de archivo | E1/E6 | 100 movimientos |
| RD-07 | Assets deduplicados | E7 | Mismo archivo 2 veces |
| RD-08 | Papelera con retención | E11 | Restaurar |
| RD-09 | Versiones comprimidas | E11 | Restaurar |
| RD-10 | Backups por hardlink | E11 | Hash + espacio |
| RD-11 | Journal de operaciones | E1/E6 | Crash a mitad |
| RD-12 | Cuarentena de corruptos | E1/E11 | JSON roto |
| RD-13 | Cifrado de archivos e índice | E11 | `strings` |
| RD-14 | Migración de esquema | E1/E11 | Fixture v1→v2 |
| RD-15 | Sin rutas absolutas en contenido | E1 | Revisión + export |

## 4. Requisitos de plugins

| ID | Requisito | Etapa | Verificación |
|---|---|---|---|
| RP-01 | Manifiesto validado | E10 | JSON Schema |
| RP-02 | Firma Ed25519 | E10 | Instalar firmado/no firmado |
| RP-03 | Permisos declarados/revocables | E10 | UI + sandbox |
| RP-04 | UI declarativa nativa | E10 + 1.0 | 3 ejemplos MVP; 5 en 1.0 |
| RP-05 | Límites CPU/RAM/UI | E10 | Bucle/1 GB |
| RP-06 | Estado persistente | E10 | Desinstalar/reinstalar |
| RP-07 | Migración de estado | E10/1.0 | `migrate` |
| RP-08 | Aislamiento de fallos | E10 | Panic/loop |
| RP-09 | SDK y plantilla | E10 | Tercero crea plugin |
| RP-10 | API congelada 1.0 | E10 | Revisión + docs |

## 5. Requisitos de configuración

| ID | Requisito | Etapa | Verificación |
|---|---|---|---|
| RC-01 | Esquema único | E3 | `ConfigSchema` |
| RC-02 | GUI generada | E3 | Todos los campos |
| RC-03 | TOML comentado | E3 | `toml_edit` |
| RC-04 | CLI get/set/edit | E3 | Roundtrip |
| RC-05 | Precedencia | E3 | Flags > env > bóveda > global |
| RC-06 | Recarga en vivo | E3 | Watcher |
| RC-07 | Error no aplica config | E3 | TOML inválido |
| RC-08 | Secretos fuera de TOML | E11 | Keyring |
| RC-09 | Paridad probada | E3 | Test de esquema |
| RC-10 | Documentación completa | E3/E13 | Tabla de claves |

## 6. Criterios de aceptación del MVP

| # | Escenario | Etapas | Evidencia |
|---|---|---|---|
| 1 | Instalación limpia | E13 | VM + Flatpak |
| 2 | Bóveda y libro de hoy | E1/E3 | Captura + test |
| 3 | Nota rica con medios | E4/E7/E8 | Nota de prueba |
| 4 | Canvas anclar/recomponer | E5 | E2E |
| 5 | Mover nota entre libros | E6 | E2E |
| 6 | Config GUI/TOML en vivo | E3 | Test |
| 7 | Plugins barra/gráfica/diagrama | E10 | Demo |
| 8 | Búsqueda y export | E9 | Golden |
| 9 | `kill -9` sin pérdida | E1/E11 | Caos |
| 10 | Rendimiento | E12 | Benchmark |

## 7. Cobertura de la visión

| Frase del usuario | Requisito | Estado |
|---|---|---|
| «cada día me cree un libro distinto» | RF-02…RF-05 | Cubierto |
| «libros de libre creación, renombrarlos» | RF-06, RF-07 | Cubierto |
| «formato día/mes/año configurable» | RF-03 | Cubierto |
| «configurable en texto como en GUI» | RF-36, RC-01…RC-10 | Cubierto |
| «notas infinitas» | RF-08 | Cubierto |
| «reordenar en modo libre» | RF-09…RF-13 | Cubierto |
| «arrastrando y soltando» | RF-10 | Cubierto |
| «entre libros distintos» | RF-10 | Cubierto |
| «mantener arriba de un libro se abre» | RF-11 | Cubierto |
| «elegir posición exacta» | RF-12 | Cubierto |
| «orden por abecedario, creación» | RF-14 | Cubierto |
| «texto, dibujos, audios, videos, webs, tablas» | RF-15…RF-21 | Cubierto |
| «formato y previsualización» | RF-22 | Cubierto |
| «plugins completamente funcionales» | RF-31…RF-35 | Cubierto |
| «barra, gráficas, diagramas» | RF-32…RF-34 | Cubierto |
| «combinar en cualquier orden, canvas» | RF-23…RF-26 | Cubierto |
| «como Word pero mejor» | RF-15, RF-22, RF-23 | Cubierto |
| «presentable, bonito» | RNF-13, E12 | Cubierto |
| «rápido» | RNF-01, RNF-03, RNF-04 | Cubierto |
| «seguro» | RNF-10…RNF-12 | Cubierto |
| «bajo consumo» | RNF-02, RNF-07, RNF-19 | Cubierto |
| «dejar trabajando, pantalla apagada, sin dormir» | RNF-21, RNF-22 | Cubierto |

## 8. Regla de mantenimiento

- Toda feature nueva debe añadir una fila a esta matriz.
- Todo bug P0/P1 debe añadir una prueba y una fila de verificación.
- En la revisión semanal, si una fila no tiene evidencia, la etapa no se considera cerrada.

## 9. Evidencia de implementación (2026-10-29)

| Requisito | Evidencia ejecutable |
|---|---|
| RF-01…RF-07 bóveda/libros diarios | `cargo test -p folio-store`, `cargo run -p folio-app -- backfill`; `docs/09` estado E1/E3 |
| RF-08…RF-14 notas y orden | `repository::tests::order_and_move_100_notes_between_books`, `reorder_within_book` |
| RF-15…RF-26 editor/bloques/lienzo | `cargo test -p folio-layout`, `cargo test -p folio-render`; smoke GTK `crates/folio-ui/tests/smoke.rs` |
| RF-27…RF-30 búsqueda/export/import | `crates/folio-app/tests/cli.rs` (FTS sin acentos, HTML sin scripts) |
| RF-31…RF-35 plugins | `cargo test -p folio-plugins` (manifiesto, firma, permisos, SSL/panic, 5 ejemplos) |
| RF-36…RF-40 configuración | `cargo test -p folio-config` (precedencia, comentarios, paridad GUI/CLI, línea de error) |
| RF-45…RF-50 entrega | `flatpak/`, `PKGBUILD`, `docs/16`, `docs/17`, `scripts/release.sh --dry-run` |
| RNF-01/03 rendimiento | `bench/baseline.json`, `scripts/perf.sh` |
| RNF-10…RNF-12 seguridad | `scripts/security.sh`, tests de assets/URL/HTML/plugins |
| RNF-18…RNF-20 recuperación | `cargo test -p folio-store backup trash versions`, `scripts/chaos.sh` |
| RNF-21…RNF-22 aislamiento | `scripts/automation/ui.sh` (Xvfb privado), `keep-awake.sh` |

### 9.4 Beta de 30 días (1.0, semana 19)

| Requisito | Evidencia |
|---|---|
| Canal beta reproducible | `scripts/beta.sh`, `dist/beta/<versión>/CHANNEL.json`, `SHA256SUMS` |
| Sin telemetría ni cliente de red | `scripts/beta-check.sh` (`cargo tree` + grep + `advanced.telemetry=false`) |
| Migraciones/campos desconocidos | escenario `unknown_fields_preserved`; rechazo de `schema` futuro en `folio-core` |
| Backup/restore y rollback | escenario `backup_verify_restore_ok`; `docs/20` protocolo |
| Triage P0/P1 | `docs/20-beta-30-dias.md`, `.github/ISSUE_TEMPLATE/beta.md` |
| Release 1.0 | `docs/21-release-1.0.md` (pendiente de firma y VM limpia) |

### 9.3 Accesibilidad y .deb (1.0, semana 18)

| Requisito | Evidencia |
|---|---|
| Teclado/foco/etiquetas | `crates/folio-ui/tests/a11y.rs`, `scripts/a11y.sh` |
| Escala 200 % y contraste de tema | `GDK_DPI_SCALE=2`, `apply_theme` claro/oscuro en el test |
| Lector de pantalla | etiquetas `Property::Label`, roles GTK; revisión manual con Orca pendiente |
| Alternativa al lienzo | `dialogs::open_block_list`, `Ctrl+Shift+B`, `Esc` vuelve al documento |
| `.deb` verificable | `scripts/build-deb.sh`, `scripts/deb_build.py`, extracción + `--version` + AppStream |
| Paquete FHS | binarios, desktop, metainfo, icono, manual, recuperación y copyright |

### 9.2 PDF (1.0, semana 17)

| Requisito | Evidencia |
|---|---|
| PDF A4 paginado | `crates/folio-render/src/pdf.rs`, `tests/pdf_golden.rs::long_note_paginates_and_media_resolves` |
| Canvas con posiciones | `write_pdf::draw_canvas_pages`, test `free_blocks_create_a_canvas_page` |
| Imágenes/assets | `AssetResolver`, test `encrypted_style_asset_callback_is_used`; integración con `Store::asset_display_path` |
| Golden PDF | `tests/golden/sample.pdf.txt` + `UPDATE_GOLDEN` |
| Export CLI/GUI | `folio export --format pdf`, acción «Exportar PDF» |
| Plugins/API | `crates/folio-plugins`, 5 manifiestos, `api = 1`; docs/06 |

### 9.1 Cifrado (1.0, semana 16)

| Requisito | Evidencia |
|---|---|
| VK, Argon2id, XChaCha20-Poly1305 | `crates/folio-store/src/crypto.rs`, tests `wrap_unlock_and_recovery`, `encrypt_decrypt_and_tamper` |
| Archivos y temporales sin texto claro | `crates/folio-store/tests/encryption.rs`, `cli_encrypted_vault_requires_password_and_leaks_no_plaintext` |
| Assets, versiones y papelera cifrados | `encryption.rs::lock_blocks_io_and_versions_trash_are_encrypted` |
| Índice reconstruible sin persistir texto | `Store::open_index`, `AppState::new`, `doctor`, tests de cifrado |
| Frase de recuperación | `Cipher::from_recovery`, test de recuperación con frase correcta/ajena |
| Auto-bloqueo y desbloqueo GUI | `App::install_auto_lock`, `dialogs::prompt_unlock`, smoke de bloqueo/desbloqueo |
| CLI/GUI sin clave en claro en logs | `commands::open_store`, `--password`/`FOLIO_PASSWORD`; no se registra la clave |
