# 08 — Calidad, pruebas y entrega

## 1. Filosofía

La calidad no se añade al final: se construye en cada etapa. Un cambio no está terminado porque «funciona en mi máquina», sino porque:

- tiene una prueba que falla sin él;
- respeta los presupuestos de rendimiento;
- no rompe datos existentes;
- es accesible por teclado;
- está documentado si cambia el comportamiento.

## 2. Pirámide de pruebas

| Nivel | Cantidad | Qué cubre | Herramientas |
|---|---|---|---|
| Unitarias | Muchas | Dominio, layout, daily, config, parsing | `cargo test` |
| Propiedades | Medias | Orden, serialización, undo, geometría | `proptest` |
| Integración | Medias | Bóveda, store, índice, migraciones | `tempfile`, fixtures |
| Golden | Pocas | Export MD/HTML/PDF, UI schema, config | snapshots |
| E2E de UI | Pocas | Flujos críticos con GTK headless | `automation/ui.sh` (Xvfb), harness |
| Caos | Puntuales | `kill -9`, disco lleno, FS ro, corrupción | scripts |
| Rendimiento | Serie | Arranque, RAM, búsqueda, canvas | `criterion`, `hyperfine` |
| Seguridad | Serie | Sandbox, sanitización, cifrado | tests + fuzz |

Objetivo de cobertura: **≥ 80 % en `folio-core`, `folio-store`, `folio-daily` y `folio-layout`**; no se persigue cobertura alta en glue de UI.

## 3. Matriz de pruebas

| Área | Unit | Integración | E2E | Caos | Rendimiento |
|---|---|---|---|---|---|
| IDs/serialización | ✓ | ✓ | | | |
| Libros diarios | ✓ | ✓ | ✓ | reloj/DST | |
| Orden de notas | ✓ | ✓ | ✓ | crash move | |
| Editor texto | ✓ | | ✓ | kill -9 | p95 |
| Canvas | ✓ | ✓ | ✓ | | 60 fps |
| Medios | | ✓ | ✓ | asset perdido | memoria |
| Web | | ✓ | | sin red | memoria |
| Plugins | ✓ | ✓ | ✓ | crash/loop | CPU/RAM |
| Búsqueda | ✓ | ✓ | ✓ | índice corrupto | latencia |
| Export | ✓ | ✓ | | | |
| Cifrado | ✓ | ✓ | ✓ | fuzz | KDF |
| Config | ✓ | ✓ | ✓ | TOML inválido | |
| Backups | | ✓ | | restore | |

## 4. Pruebas de dominio

### 4.1 Orden de notas

- Insertar al principio, medio y final.
- Mover entre libros vacíos, con una nota y con muchas.
- Mover la misma nota dos veces seguidas.
- Colisión de claves y rebalanceo.
- 10,000 movimientos aleatorios con `proptest`: el orden resultante es una permutación válida, sin duplicados ni pérdidas.
- Crash simulado en cada paso del journal: al recuperar, la nota está exactamente en un libro.

### 4.2 Día lógico

- Zona `Europe/Madrid`, `America/New_York`, `UTC`.
- Cambio de horario de verano: 23 h y 25 h.
- Hora de corte 0, 4 y 23.
- Abrir a las 03:59 y 04:00.
- Cambio de zona del sistema con la app abierta.
- Reloj hacia atrás/adelante.
- Libro renombrado: no se duplica.
- Libro borrado: no se recrea por defecto.
- Backfill con rango y sin duplicados.

### 4.3 Serialización

- Roundtrip de cada tipo de bloque.
- Campos desconocidos preservados.
- Bloque de plugin ausente preservado.
- JSON inválido → cuarentena, no pérdida.
- Schema futuro → negarse a abrir con mensaje claro.
- Migración de fixture v1 → v2.

### 4.4 Undo

- 100 pasos de texto, formato, bloques, canvas y DnD.
- Deshacer después de guardar.
- Rehacer tras una nueva acción (se limpia la rama).
- Coalescing de tecleo: una palabra o ráfaga = un paso razonable.
- Undo de un movimiento entre libros restaura libro y posición.

## 5. Pruebas de UI

- Arranque con bóveda vacía, con bóveda grande, con bóveda bloqueada.
- Crear libro/nota, renombrar, borrar, restaurar.
- Arrastrar nota dentro y entre libros; spring-load; selector; cancelar con `Esc`.
- Canvas: anclar, mover, recomponer, zoom, selección múltiple, teclado.
- Previsualización y concentración.
- Ajustes: cambiar un valor, recargar, error TOML.
- Paleta de comandos: búsqueda y ejecución.
- Plugin: insertar, interactuar, desactivar, marker de ausencia.
- Accesibilidad: recorrido con Tab, foco visible, nombres accesibles.

Herramientas:

- `gtk4` test harness + `dogtail`/`accerciser` para árbol accesible.
- Capturas con `grim` para regresión visual (comparación perceptual con umbral).
- No se hacen pruebas de píxel frágiles; se comparan regiones clave.

### 5.1 Aislamiento obligatorio

Los tests de UI se ejecutan con:

```bash
./scripts/automation/ui.sh
```

Ese comando arranca un **Xvfb privado**, un bus D-Bus propio y `HOME`/XDG temporales. Nunca usa el `DISPLAY` del usuario, no abre ventanas en la sesión real y no toca `~/.config`. El detalle está en [15-automatizacion.md](15-automatizacion.md).

- Un test que use `sleep` o `~/Notas` se rechaza.
- GTK corre con `--test-threads=1`; si se paraleliza, cada worker tiene su Xvfb y su `HOME`.
- Los artefactos (log, entorno, capturas) van a `.artifacts/` y se suben en CI si falla.
- Xvfb es X11: el smoke de Wayland se hace manualmente antes de release.
- Para suites largas: `./scripts/automation/keep-awake.sh -- ./scripts/automation/all.sh` evita la suspensión y permite apagar la pantalla. `nightly.sh` lo hace con log propio; ver [15-automatizacion.md](15-automatizacion.md).

## 6. Pruebas de caos

Script `scripts/chaos.sh`:

1. Bóveda de 1,000 notas.
2. Bucle de edición + guardado.
3. `kill -9` en momentos aleatorios (10 rondas).
4. Reabrir, ejecutar `doctor`, comprobar:
   - todas las notas parsean;
   - no hay duplicados;
   - el índice se reconstruye;
   - el contenido de la última versión guardada está intacto.
5. Disco lleno con `fallocate`/tmpfs pequeño; comprobar error y recuperación.
6. FS de solo lectura; comprobar modo lectura.
7. Borrar `index.sqlite`; reconstruir.
8. Corromper un JSON; cuarentena y resto intacto.
9. Mover nota y matar en cada paso del journal.
10. Restaurar backup y comparar hashes.

Criterio: **cero pérdidas y cero corrupciones silenciosas**.

## 7. Pruebas de rendimiento

- `folio-tools gen-vault` genera 10k notas con texto, tablas, imágenes y plugins.
- `hyperfine` mide arranque frío/caliente.
- `criterion` mide layout, serialización, búsqueda, orden, canvas hit-testing.
- `heaptrack`/`massif` miden memoria; se comparan picos.
- `perf` mide frames y CPU.
- La CI guarda resultados y compara con `baseline.json`.
- Un cambio que empeora > 20 % requiere justificación y aprobación.

## 8. Pruebas de seguridad

- Fuzz de parser JSON/TOML, manifiesto de plugin, importador Markdown, HTML sanitizer.
- Intento de path traversal en assets y plugins.
- Plugin sin permisos llama a host functions prohibidas.
- Plugin con bucle infinito/memoria excesiva.
- WebView sin cookies/descargas/permisos.
- Bóveda cifrada: `strings`, temporales, logs, crash dump.
- Enlaces peligrosos (`javascript:`, `data:`, `file:`).
- SVG con script/`foreignObject`.
- Dependencias: `cargo audit`, `cargo deny`.

## 9. CI

### 9.1 Pipeline mínimo por PR

```bash
./scripts/automation/all.sh
```

Equivale a:

```bash
./scripts/automation/plan.sh
./scripts/automation/quality.sh          # fmt + clippy + deny/audit
./scripts/automation/unit.sh             # dominio/store sin GTK
./scripts/automation/ui.sh               # UI dentro de Xvfb privado
```

`all.sh --fast` omite la UI cuando se itera en lógica de dominio. La UI nunca se ejecuta en el display del usuario.

### 9.2 Pipeline nocturno

```bash
./scripts/automation/nightly.sh
cargo test --workspace --release
cargo llvm-cov --workspace --fail-under-lines 70
cargo bench --workspace -- --save-baseline ci
./scripts/chaos.sh
cargo fuzz run parse_note -- -max_total_time=300
cargo fuzz run plugin_manifest -- -max_total_time=300
flatpak-builder --force-clean build-dir flatpak/io.github.leonisauro.Folio.yml
```

### 9.3 Matriz

| Job | Entorno |
|---|---|
| Linux estable | Arch, Rust stable |
| Linux mínimo | Ubuntu 24.04 (GTK 4.14) |
| Flatpak | Runtime GNOME actual |
| Sin WebKit | Build con `--no-default-features` |
| Sin GStreamer | Build con feature desactivada |
| Docs | `mdbook`/links + `check_plan.py` |

## 10. Definición de terminado (DoD)

Una tarea está terminada cuando:

- [ ] El código compila sin warnings.
- [ ] `fmt` y `clippy` pasan.
- [ ] Tiene pruebas unitarias/integración según corresponda.
- [ ] Los tests de UI se ejecutaron con `automation/ui.sh` en Xvfb, no en el display del usuario.
- [ ] La prueba falla antes del cambio y pasa después (cuando es un bug).
- [ ] No rompe migraciones ni fixtures.
- [ ] Respeta los presupuestos de rendimiento.
- [ ] Es accesible por teclado y tiene nombre accesible.
- [ ] Los textos de UI están en Fluent (es) y no hardcodeados.
- [ ] La configuración nueva está en el esquema, GUI, CLI y docs.
- [ ] La documentación se actualizó (si cambia comportamiento).
- [ ] Se probó en una bóveda real y en una vacía.
- [ ] Se actualizó el estado de la etapa y el changelog.

## 11. Severidad de bugs

| Nivel | Definición | Respuesta |
|---|---|---|
| P0 | Pérdida/corrupción de datos, ejecución de código, bypass de sandbox | Bloquea release; fix inmediato |
| P1 | Crash reproducible, función esencial rota, accesibilidad bloqueante | Bloquea release |
| P2 | Bug importante con workaround, rendimiento fuera de presupuesto | Arreglar antes de beta |
| P3 | Molestia visual, caso límite | Backlog |
| P4 | Mejora, texto, detalle | Backlog |

## 12. Beta

- 5–10 usuarios, idealmente con bóvedas reales y copia de seguridad.
- Canal Flatpak beta + binario firmado.
- Formulario local de feedback (exportable) sin telemetría.
- Beta inicial de 7 días para el MVP; beta de 30 días para 1.0; diario de incidencias.
- Criterio de salida del MVP: 7 días sin P0/P1, rendimiento en presupuesto, backup/restore probado y accesibilidad básica. Para 1.0: 30 días y auditoría.

## 13. Checklist de release

### Código
- [ ] Versión y changelog.
- [ ] `Cargo.lock` actualizado.
- [ ] Migraciones probadas desde la versión anterior.
- [ ] `cargo deny`/`audit` limpios.
- [ ] Tests y benchmarks verdes.
- [ ] `check_plan.py` verde.

### Datos
- [ ] Backup/restore realizado en una bóveda real.
- [ ] `doctor` sobre bóveda grande sin errores.
- [ ] Prueba de caos pasada.
- [ ] Cifrado probado (si aplica).

### Producto
- [ ] Instalación limpia en Arch/GNOME.
- [ ] Flatpak con permisos mínimos.
- [ ] AUR generado en el MVP; `.deb` en 1.0.
- [ ] AppStream, iconos, capturas.
- [ ] Manual de usuario y de plugins.
- [ ] Atajos y accesibilidad revisados.
- [ ] Sin texto en inglés en la UI española.
- [ ] Enlaces de descarga y firmas.

### Distribución
- [ ] Tag firmado.
- [ ] Binario reproducibilidad verificada.
- [ ] Flatpak publicado en el remoto beta/estable.
- [ ] AUR actualizado.
- [ ] Notas de versión con cambios y limitaciones.
- [ ] Plan de rollback.

## 14. Post-release

- Monitorizar issues P0/P1.
- Revisar logs de crash opt-in.
- Publicar parche en ≤ 7 días para P0/P1.
- Retrospectiva de release y actualización del plan.
- No añadir features hasta estabilizar.
