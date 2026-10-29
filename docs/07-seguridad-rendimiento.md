# 07 — Seguridad, privacidad y rendimiento

## 1. Modelo de amenazas

### 1.1 Activos

- Contenido de notas, dibujos, medios y adjuntos.
- Metadatos: títulos, fechas, libros, patrones de uso.
- Claves de cifrado y contraseña de bóveda.
- Configuración y permisos de plugins.
- Integridad del sistema de archivos de la bóveda.

### 1.2 Actores y vectores

| Actor | Vector | Objetivo |
|---|---|---|
| Contenido no confiable | HTML pegado, SVG, Markdown importado, metadatos de enlaces | Inyección, XSS, ejecución |
| Plugin malicioso o defectuoso | Wasm, UI declarativa, peticiones de red | Robo de datos, agotamiento, persistencia |
| Sitio web cargado | WebKit, cookies, permisos, descargas | Fuga de sesión, tracking, exploit |
| Atacante local | Archivos de la bóveda, backups, logs, portapapeles | Lectura de notas |
| Cadena de suministro | Dependencias Rust, runtime Flatpak, plugins sin firma | Compromiso de la app |
| Fuerza bruta | Bóveda cifrada robada | Descifrado |
| Error del usuario | Borrado, sobrescritura, mover a otro FS | Pérdida de datos |
| Fallo del sistema | Corte de energía, disco lleno, FS de solo lectura | Corrupción |

### 1.3 Fuera del modelo

- Un atacante con control total del kernel o del usuario mientras la bóveda está desbloqueada.
- Malware que lee la memoria del proceso.
- Cámaras/micrófonos comprometidos.
- Servicios externos que el usuario decida usar (sync futuro).

## 2. Controles por capa

### 2.1 Proceso y plataforma

- Rust seguro por defecto; `unsafe` solo en wrappers FFI documentados y revisados.
- Flatpak con sandbox; permisos mínimos: acceso a la carpeta de bóveda elegida por portal, no a todo el home por defecto.
- Sin setuid, sin demonios, sin servicios en segundo plano.
- Instancia única por D-Bus; sin sockets abiertos salvo el bus de sesión.
- ASLR/PIE del sistema; binario no ejecuta código generado en tiempo de ejecución salvo Wasm aislado.

### 2.2 Sistema de archivos

- `cap-std`/`openat2(RESOLVE_BENEATH)` para operaciones dentro de la bóveda.
- Canonicalización y rechazo de `..`, enlaces simbólicos que escapan, dispositivos y FIFOs.
- Escritura atómica con `fsync`; permisos `0700` para directorios y `0600` para archivos.
- Los assets se abren con la app del sistema solo tras confirmación; nunca se ejecutan.
- Límites de tamaño y tipo al importar; SVG saneado; imágenes decodificadas con límites.
- Nombres de archivo saneados; los títulos no se usan como rutas.
- La papelera y los backups viven dentro de la bóveda, con los mismos permisos.

### 2.3 Contenido

- El texto se renderiza con Pango, nunca con HTML del usuario.
- El HTML pegado/importado se sanea con `ammonia` (whitelist) y se convierte a spans/bloques.
- Los enlaces se validan; `javascript:`, `data:`, `file:` se rechazan.
- Las imágenes se decodifican con límites de píxeles y memoria; se respeta EXIF solo para orientación.
- Los SVG se sanean o se rasterizan; sin scripts, `foreignObject` ni referencias externas.
- La exportación HTML escapa todo y usa una CSP estricta si se abre en navegador.
- PDF se genera sin JavaScript.

### 2.4 Web

- Tarjeta por defecto; la vista viva es una acción explícita.
- WebKitGTK en proceso separado; `WebsiteDataManager` efímero en memoria.
- Sin cookies persistentes, sin almacenamiento local persistente, sin descargas, sin popups, sin permisos de cámara/micrófono/geolocalización/notificaciones.
- Enlaces externos se abren en el navegador del sistema, no en la app.
- La web viva se descarga al salir de pantalla y al cerrar la nota.
- No se renderiza web viva en PDF/HTML exportado.
- Límite de tiempo de carga y de memoria observado; si WebKit falla, se muestra tarjeta.
- Feature flag: si WebKit no está, la app sigue siendo completa.

### 2.5 Plugins

- WASM sin WASI ambiental; solo host functions.
- Permisos por capacidad, declarados y revocables.
- Límites de memoria, fuel, epoch, tamaño de UI, storage y red.
- Firma Ed25519; `allow_unsigned` desactivado por defecto.
- Validación estricta del manifiesto, árbol UI y estado.
- Aislamiento de fallos: un plugin caído no afecta a la nota ni a la app.
- Registro de auditoría de permisos y llamadas sensibles.
- Detalle en [06-plugins.md](06-plugins.md).

### 2.6 Cadena de suministro

- `Cargo.lock` commiteado; builds reproducibles en lo posible.
- `cargo deny` (licencias, advisories, duplicados) y `cargo audit` en CI.
- Dependencias mínimas y revisadas; `wasmtime`, `webkit2gtk`, `gstreamer` son las de mayor superficie.
- Flatpak con runtime actualizado; releases firmadas.
- AUR en el MVP y `.deb` en 1.0, generados desde el mismo commit y verificados por hash.
- Sin descargas de código en tiempo de ejecución; los plugins se instalan manualmente.
- Revisión de advisories de GTK/WebKit y plan de actualización.

### 2.7 Privacidad

- Sin telemetría por defecto. Si se añade, opt-in, agregada y documentada.
- Sin cuentas, sin nube, sin analytics.
- Los logs no contienen texto de notas ni rutas completas por defecto; `advanced.log_content` es explícito.
- Los metadatos de enlaces solo se piden bajo demanda o con preferencia explícita.
- La búsqueda es local.
- Los crash reports son locales y opt-in para enviar.
- El portapapeles se puede limpiar tras N segundos (1.1); en MVP, aviso al copiar contenido de bóveda cifrada.

## 3. Cifrado de bóveda

> Diseño detallado en [03-modelo-datos.md](03-modelo-datos.md), sección 13. La implementación entra en 1.0 (semanas 15–20); el MVP ya incluye backups, papelera y sandbox.

| Aspecto | Decisión |
|---|---|
| KDF | Argon2id, 64 MiB, t=3, p=4, salt 16 B |
| Cifrado | XChaCha20-Poly1305, nonce 24 B por archivo/bloque |
| Clave | 32 B aleatoria, envuelta con KEK |
| Metadatos | Nombres opacos, títulos/fechas cifrados |
| Índice | SQLCipher o contenedor cifrado; FTS tras desbloqueo |
| Bloqueo | Inactividad, suspensión, logind `Lock` |
| Memoria | `zeroize` al bloquear; sin swap de claves |
| Recuperación | Clave de 24 palabras mostrada una vez |
| Backups | Permanecen cifrados; sin clave |
| Búsqueda | Solo tras desbloquear; sin índice en claro |
| Plugins | No reciben la clave; el host descifra y entrega datos según permiso |

Pruebas:

- El texto plano no aparece en archivos temporales, logs, caché ni swap del proceso.
- `strings` sobre la bóveda no revela títulos ni contenido.
- Cambiar un byte produce error de autenticación, no corrupción silenciosa.
- Argon2 tarda 200–500 ms en hardware de referencia; se muestra progreso.
- Contraseña incorrecta no distingue «clave mala» de «archivo corrupto» en el mensaje externo.

## 4. Presupuestos de rendimiento

### 4.1 Hardware y método

- Referencia: 4 núcleos, 8 GB RAM, SSD SATA, 1080p, GNOME/Wayland.
- Medición en CI con `hyperfine`, `criterion`, instrumentación interna y `heaptrack`.
- Bóveda sintética: 10,000 notas, 100,000 bloques, 500 assets, 1 GB de medios.
- Cada release compara contra la línea base; una regresión > 20 % falla el CI.

### 4.2 Tabla de objetivos

| Métrica | Objetivo | Límite duro |
|---|---|---|
| Arranque frío | < 600 ms | 1 s |
| Arranque caliente | < 250 ms | 500 ms |
| RAM en reposo con bóveda grande | < 150 MB | 250 MB |
| Abrir nota 5k palabras | < 100 ms | 250 ms |
| Tecleo p95 | < 16 ms | 33 ms |
| Paneo/zoom 500 bloques | 60 fps | 30 fps |
| Búsqueda 10k notas | < 80 ms | 200 ms |
| Guardado | < 30 ms | 100 ms |
| Indexación completa 10k | < 30 s | 60 s |
| CPU reposo | < 0,5 % | 2 % |
| Binario | < 35 MB | 50 MB |
| Plugin render | < 16 ms | 50 ms |
| Memoria plugin | < 64 MB | 128 MB |

### 4.3 Estrategias

**Arranque**
- No cargar notas hasta que la ventana es visible.
- Índice SQLite con consultas mínimas; lista virtualizada.
- Plugins y WebKit cargados bajo demanda.
- Sin comprobaciones de red al inicio.
- Lazy loading de fuentes/iconos.

**Memoria**
- Listas y canvas virtualizados; solo widgets visibles.
- Cachés LRU con límites (texto, miniaturas, ondas, texturas).
- Destruir WebViews y reproductores al salir de pantalla.
- Streaming de audio/vídeo; no cargar el archivo completo.
- Evitar duplicar el JSON y el modelo: una representación canónica.

**Tecleo**
- Modelo en memoria; autosave debounced.
- Layout de texto incremental; no re-medir toda la nota por tecla.
- Tags de GTK para formato; sin reconstruir el buffer.
- Undo coalescido; sin serializar en cada pulsación.

**Canvas**
- Solo bloques visibles + margen.
- Texturas de dibujo cacheadas por zoom.
- Layout puro y rápido; sin asignaciones por frame.
- Repintado por daño, no global.
- LOD a zoom bajo.

**Búsqueda**
- FTS5; índice incremental; consultas preparadas.
- Debounce de 120 ms en la UI; cancelación de búsquedas anteriores.
- Resultados limitados a 50 con paginación.

**Medios**
- GStreamer con `playbin3`; pausa offscreen.
- Miniaturas y waveforms precalculadas en background.
- Límite de píxeles al decodificar imágenes.
- `GdkTexture` liberada al salir de pantalla.

**Plugins**
- Instancia por bloque visible; destruir al salir.
- Render throttle 60 Hz; estado debounced.
- Sin recarga si el árbol no cambió (hash).
- Límites duros de memoria/CPU.

**Indexación**
- Hilo dedicado, lotes de 100, checkpoint.
- Reindexado incremental por mtime/tamaño/hash.
- Reconstrucción en base temporal + swap atómico.
- No bloquear la UI; progreso cancelable.

### 4.4 Bajo consumo

- Sin timers de polling; todo event-driven.
- Sin animaciones cuando la ventana no está visible.
- Sin indexación si no hay cambios.
- Sin red en segundo plano.
- WebKit/GStreamer pausados fuera de pantalla.
- Modo «ahorro» opcional (1.1): desactiva miniaturas animadas y precarga.

## 5. Pruebas de rendimiento

```bash
# Arranque
hyperfine --warmup 3 './target/release/folio --headless --quit-after-init'

# Benchmarks de dominio
cargo bench -p folio-layout -p folio-core -p folio-store

# Memoria
heaptrack ./target/release/folio --open-vault tests/fixtures/vault-10k

# Bóveda sintética
cargo run -p folio-tools -- gen-vault --notes 10000 --out /tmp/folio-10k

# Perfil de UI
perf record -g ./target/release/folio
```

- `folio-tools` es un crate interno de desarrollo; no se distribuye.
- Los benchmarks se ejecutan en CI sobre un runner dedicado o se comparan con la línea base local.
- Las regresiones se revisan antes de fusionar.

## 6. Pruebas de seguridad

| Prueba | Esperado |
|---|---|
| Nota con `<script>` en HTML pegado | Se elimina; solo texto |
| Enlace `javascript:` | Rechazado |
| Asset `.svg` con script | Saneado/rasterizado |
| Plugin sin `network` llama `http-get` | Error `denied` |
| Plugin intenta `../` en asset | Rechazado |
| Plugin bucle infinito | Interrumpido a los 2 s |
| Plugin 1 GB de UI | Rechazado |
| WebView con cookie | No persiste |
| WebView descarga | Bloqueada |
| Bóveda cifrada + `strings` | Sin texto plano |
| Backup cifrado | Sin clave |
| Archivo corrupto | Cuarentena, no borrado |
| Symlink fuera de la bóveda | Rechazado |
| Disco lleno | Error claro, sin corrupción |
| FS solo lectura | Modo lectura, sin pérdida |

## 7. Respuesta a incidentes

1. **Plugin malicioso**: desactivar, revocar permisos, conservar evidencia en log, avisar.
2. **Vulnerabilidad en dependencia**: advisory, parche, release; si es crítico, mitigación temporal documentada.
3. **Pérdida de datos**: no sobrescribir; usar backups/versiones; `doctor`; guía de recuperación.
4. **Bóveda corrupta**: cuarentena, reconstruir índice, restaurar backup, exportar lo recuperable.
5. **Fallo de WebKit**: descargar vista viva, deshabilitar feature flag, actualizar runtime.

## 8. Criterios de aceptación

- [ ] El modelo de amenazas está documentado y revisado en E11 y en la etapa de cifrado de 1.0.
- [ ] `cargo deny` y `cargo audit` pasan sin advisories críticos.
- [ ] Todas las pruebas de seguridad de la tabla pasan.
- [ ] La bóveda cifrada no deja texto plano en temporales/logs.
- [ ] Los presupuestos de rendimiento se miden en CI.
- [ ] Una regresión > 20 % bloquea el release.
- [ ] La app funciona sin WebKit y sin GStreamer (con degradación).
- [ ] El usuario puede ver y revocar permisos de plugins.
- [ ] No hay telemetría ni red por defecto.
