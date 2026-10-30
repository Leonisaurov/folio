# 10 — Registro de decisiones (ADR)

Una ADR por decisión relevante. Formato: contexto → decisión → consecuencias → alternativas. Las decisiones cerradas con el usuario (D-01…D-12) se reflejan aquí como ADR.

---

## ADR-001 — Rust + GTK4/Libadwaita

**Contexto**: se necesita una app Linux nativa, rápida, de bajo consumo, bonita y con control fino de UI y canvas.

**Decisión**: Rust estable + GTK4 + Libadwaita, con Pango para texto y GSK para el canvas. Sin Electron ni WebView para el editor.

**Consecuencias**:
- Arranque y RAM competitivos; integración GNOME; seguridad de memoria.
- Más trabajo de UI que en web; el editor híbrido es el mayor riesgo.
- Se depende del ritmo de GTK/Libadwaita y de sus versiones mínimas.

**Alternativas**: Tauri (más rápido de desarrollar, más RAM); Qt/QML (maduro, C++ más costoso); Flutter (no nativo, consumo).

---

## ADR-002 — Archivos como fuente de verdad, SQLite como índice

**Contexto**: portabilidad, recuperación ante corrupción y confianza del usuario.

**Decisión**: libros/notas en JSON versionado y assets en archivos; SQLite/FTS5 solo como índice reconstruible.

**Consecuencias**:
- Datos inspeccionables y respaldables sin la app.
- Se paga complejidad de mantener archivos e índice sincronizados.
- El índice se puede borrar y reconstruir; nunca es la verdad.

**Alternativas**: todo en SQLite (simple, opaco); Markdown puro (portable, insuficiente para canvas/plugins).

---

## ADR-003 — Nota = lienzo infinito con flujo por defecto

**Contexto**: el usuario quiere libertad espacial sin perder la comodidad de un documento lineal.

**Decisión**: cada nota tiene un layout híbrido. Los bloques están en flujo por defecto; al arrastrarlos se anclan (`placement = free`). «Recomponer» los devuelve al flujo; «Volver al flujo» reinserta uno.

**Consecuencias**:
- Se obtiene escritura cómoda y libertad en el mismo documento.
- El layout, el hit-testing y el undo son más complejos.
- Los bloques anclados dejan un hueco en el flujo; es intencional.

**Alternativas**: solo lienzo (menos cómodo); dos vistas separadas (menos integradas); documento clásico (sin libertad).

---

## ADR-004 — Pertenencia por carpeta y orden por nombre de archivo

**Contexto**: mover notas entre libros debe ser atómico y no duplicar.

**Decisión**: la nota vive en `books/<book>/notes/<order_key>_<id>.json`; la carpeta define el libro y el prefijo define el orden manual. Mover = `rename` atómico.

**Consecuencias**:
- Una sola operación de FS; sin manifiestos duplicados.
- Reordenar renombra el archivo; los enlaces usan el ID, no la ruta.
- La bóveda debe estar en un solo sistema de archivos.

**Alternativas**: manifiesto por libro (dos escrituras); base de datos para membresía (opaco).

---

## ADR-005 — Bloques de texto con secciones, no un documento monolítico

**Contexto**: el canvas necesita bloques movibles, pero la escritura necesita continuidad.

**Decisión**: un bloque `text` contiene varios párrafos con estilos; los bloques no texto son unidades separadas. Insertar un bloque no texto parte la sección de texto en el cursor.

**Consecuencias**:
- Menos bloques y mejor rendimiento que un bloque por párrafo.
- Mover una sección mueve toda la prosa; para granularidad fina se puede dividir.
- El editor mantiene continuidad dentro de la sección.

**Alternativas**: bloque por párrafo (más granular, más overhead); un solo documento (sin canvas).

---

## ADR-006 — Editor con GtkTextView por bloque (con spike y fallback)

**Contexto**: IME, accesibilidad, undo y rendimiento son críticos; un editor propio desde cero es caro.

**Decisión**: implementar la opción A (GtkTextView por bloque) tras un spike de 3 días. Si falla, usar la opción C (GtkTextView único + canvas de previews) para el MVP.

**Consecuencias**:
- Se reutiliza la infraestructura madura de GTK.
- La selección/copia entre bloques y la sincronización con el modelo requieren trabajo.
- La opción B (editor propio con Pango) queda para 1.0+ si hace falta.

**Alternativas**: editor propio (control total, meses); WebView (rápido, pesado).

---

## ADR-007 — Libro diario como libro normal con `daily_date`

**Contexto**: el usuario quiere libros diarios automáticos pero también renombrarlos y tratarlos como cualquier libro.

**Decisión**: un libro diario es un `Book` con `kind = "daily"` y `daily_date`; el título es presentación y se puede renombrar. La unicidad es por `daily_date`, no por título.

**Consecuencias**:
- Renombrar no duplica; el libro participa en todas las funciones.
- Si se borra, no se recrea por defecto.
- El título automático se aplica solo al crear o al restaurar.

**Alternativas**: carpeta especial para diarios (duplica lógica); título como clave (frágil).

---

## ADR-008 — Configuración TOML + GUI generada + CLI

**Contexto**: el usuario exige configuración total por texto y GUI, sin divergencias.

**Decisión**: un `ConfigSchema` único genera validación, formulario, JSON Schema, CLI y docs. `toml_edit` preserva comentarios; la GUI solo escribe claves cambiadas; recarga en vivo.

**Consecuencias**:
- Paridad garantizada por pruebas.
- El esquema es una API pública que hay que versionar.
- Los secretos van al keyring, no al TOML.

**Alternativas**: configuración solo GUI (no cumple); solo TOML (no cumple); base de datos (opaca).

---

## ADR-009 — Plugins WASM con UI declarativa nativa

**Contexto**: plugins funcionales sin comprometer seguridad ni rendimiento.

**Decisión**: Wasmtime + Component Model/WIT; UI como árbol declarativo renderizado con GTK; permisos por capacidad; sin HTML/CSS/JS de plugin.

**Consecuencias**:
- Sandbox fuerte y bajo overhead.
- La UI de plugin está limitada al catálogo de componentes; plugins muy visuales necesitan `canvas`.
- Hay que mantener un SDK y una API estable.

**Alternativas**: WebView HTML (flexible, pesado/inseguro); plugins nativos (sin sandbox); scripts embebidos (inseguro).

---

## ADR-010 — Permisos explícitos y sin autoridad ambiental

**Contexto**: un plugin no debe poder leer archivos, abrir sockets ni borrar notas por defecto.

**Decisión**: el plugin solo accede a funciones del host; cada permiso se declara, se muestra al instalar y se puede revocar. Sin WASI ambiental.

**Consecuencias**:
- Más fricción de instalación, mucha más seguridad.
- El host debe implementar y auditar cada función.
- Los plugins que necesitan red piden `network` con allowlist.

**Alternativas**: confiar en el autor (no); sandbox del sistema operativo (complemento, no sustituto).

---

## ADR-011 — Web tarjeta por defecto, vista viva opcional

**Contexto**: incrustar webs es útil pero pesado y arriesgado.

**Decisión**: tarjeta de enlace con metadatos cacheados; vista viva solo bajo demanda, con perfil efímero, sin cookies/descargas/permisos y sin renderizarse en exportaciones.

**Consecuencias**:
- La app es ligera y segura por defecto.
- Algunas webs no se verán completas hasta pulsar «Cargar».
- WebKit es una dependencia opcional.

**Alternativas**: siempre en vivo (pesado); solo tarjetas (menos útil).

---

## ADR-012 — Cifrado opcional por bóveda

**Contexto**: privacidad sin imponer una contraseña a todos.

**Decisión**: bóveda normal o cifrada con Argon2id + XChaCha20-Poly1305; clave envuelta; bloqueo automático; clave de recuperación; índice cifrado.

**Consecuencias**:
- El usuario elige el equilibrio seguridad/comodidad.
- La búsqueda requiere desbloquear; hay que gestionar recuperación.
- Los backups permanecen cifrados.

**Alternativas**: cifrado obligatorio (fricción); sin cifrado (menos seguro).

---

## ADR-013 — Local primero, CRDT-ready

**Contexto**: la sincronización no entra en el MVP, pero no debe requerir reescribir el modelo.

**Decisión**: IDs UUIDv7 estables, operaciones de dominio y datos sin dependencia de rutas; la sync se decide en la etapa 14 y se implementará con CRDT (Automerge/Loro) sobre el mismo modelo.

**Consecuencias**:
- No se paga complejidad de sync ahora.
- Hay que evitar supuestos de un solo escritor en el modelo (aunque el MVP lo sea).
- La sync futura necesitará resolver assets y plugins.

**Alternativas**: sync desde V1 (retrasa todo); nunca sync (limita).

---

## ADR-014 — Sin WebView para el editor

**Contexto**: la tentación de usar Tauri/WebView para el editor es alta.

**Decisión**: el documento se renderiza con GTK/Pango; WebKit solo para web incrustada.

**Consecuencias**:
- Menor RAM, arranque más rápido, integración nativa.
- Más esfuerzo de editor.
- Se evita la superficie de ataque de un motor web para todo el contenido.

**Alternativas**: editor web (rápido de iterar, peor consumo); editor propio completo (caro).

---

## ADR-015 — Instancia única y CLI sobre D-Bus

**Contexto**: abrir varias instancias puede corromper la bóveda; la CLI debe funcionar con la app abierta.

**Decisión**: `GtkApplication` con ID único; la CLI usa acciones D-Bus; deep links `folio://`.

**Consecuencias**:
- Una sola ventana escribe; la segunda activa la existente.
- La CLI depende del bus de sesión; sin bus, modo headless.
- Los enlaces del sistema abren la nota correcta.

**Alternativas**: múltiples instancias con locks (más frágil); CLI standalone (no sincroniza estado).

---

## ADR-016 — Backups por hardlinks, papelera y versiones

**Contexto**: la integridad es crítica y los backups completos duplican medios.

**Decisión**: snapshots por hardlinks dentro de la bóveda; papelera con retención; versiones comprimidas por nota; restauración sobre copia.

**Consecuencias**:
- Backups baratos en el mismo FS; no protegen contra fallo del disco (se recomienda copia externa).
- Los hardlinks requieren mismo FS.
- El usuario debe entender que el backup no sustituye una copia externa.

**Alternativas**: zip completo (caro); backup externo automático (fuera de alcance).

---

## ADR-017 — i18n con Fluent, español primero

**Contexto**: la UI debe ser española y preparada para inglés.

**Decisión**: Fluent para textos; fechas con ICU/jiff; sin strings hardcodeados; plurales y géneros en el archivo de idioma.

**Consecuencias**:
- Traducción posterior sencilla.
- Un paso extra en cada texto de UI.
- Pruebas de que no quedan claves sin traducir.

**Alternativas**: gettext (válido); strings directos (no).

---

## ADR-018 — Flatpak primario, AUR y deb

**Contexto**: distribución Linux heterogénea y sandbox.

**Decisión**: Flatpak como canal principal (runtime GNOME, permisos mínimos); AUR en el MVP y `.deb` en 1.0, generados del mismo commit; AppStream y firmas.

**Consecuencias**:
- Instalación consistente y sandbox.
- El acceso a bóvedas usa portales; hay que probar rutas.
- AUR/deb requieren dependencias del sistema (GTK, WebKit, GStreamer).

**Alternativas**: solo binario tar.gz (simple, sin sandbox); solo AUR (Arch-only).

---

## ADR-019 — Teclado primero y accesibilidad del canvas

**Contexto**: el canvas puede excluir a usuarios de teclado/lector de pantalla.

**Decisión**: toda acción tiene atajo/menú; el canvas ofrece vista de lista y movimiento por flechas; auditoría en E12.

**Consecuencias**:
- Más trabajo de UI y pruebas.
- Mejor producto para todos.
- El canvas no es la única forma de hacer nada.

**Alternativas**: accesibilidad después (deuda cara); solo ratón (excluyente).

---

## ADR-020 — Presupuestos de rendimiento con CI

**Contexto**: «rápido y de bajo consumo» debe ser verificable, no una intención.

**Decisión**: presupuestos numéricos (arranque, RAM, tecleo, búsqueda, canvas), benchmarks y comparación contra baseline; regresión > 20 % bloquea.

**Consecuencias**:
- Se detectan regresiones temprano.
- La CI necesita hardware/tiempo; se acepta.
- Algunas features se recortan si no caben.

**Alternativas**: medición manual (se olvida); sin presupuestos (se degrada).

---

## ADR-021 — Nombres y licencia (provisional)

**Contexto**: el proyecto necesita identidad y licencia antes de publicar.

**Decisión provisional**: nombre «Folio»; app GPL-3.0-or-later; SDK de plugins MIT OR Apache-2.0. Confirmar antes de la beta.

**Consecuencias**:
- La GPL protege la libertad del usuario y obliga a abrir derivados.
- El SDK permisivo fomenta plugins.
- Cambiar de nombre antes de la beta es barato; después, no.

**Alternativas**: MIT para todo (más permisivo); propietario (fuera del espíritu local-first).

---

## ADR-022 — Tests de UI en Xvfb privado y trabajo desatendido

**Contexto**: ejecutar GTK en el display del usuario roba foco, abre ventanas encima del trabajo y falla si la sesión está bloqueada. Además, una suite larga no debe suspenderse al apagarse la pantalla.

**Decisión**: todos los tests de UI se ejecutan con `scripts/automation/with-x11.sh`: Xvfb privado (`xvfb-run -a`), D-Bus propio, `HOME`/XDG temporales, backend X11 y renderizado software. `keep-awake.sh` usa `systemd-inhibit --what=sleep` (y opcionalmente `handle-lid-switch`) para evitar la suspensión, **sin inhibir `idle`**, de modo que la pantalla puede apagarse.

**Consecuencias**:
- Las pruebas no interrumpen la sesión y pueden dejarse corriendo.
- Xvfb cubre X11, no Wayland: se mantiene un smoke manual en GNOME/Wayland.
- Los benchmarks de FPS/latencia se miden en sesión real, no en Xvfb.
- Hace falta `xvfb`, `dbus` y `systemd`/logind; sin ellos se avisa y se degrada.
- La suite corre con `--test-threads=1`; paralelizar exige un Xvfb y `HOME` por worker.

**Alternativas**: usar el display real (rechazado); Weston headless (futuro para Wayland); no probar UI (rechazado).

---

## ADR-023 — Runtime de plugins: contrato primero y referencia en Rust

**Fecha**: 2026-10-29
**Estado**: aceptada para el MVP; revisar en 1.0

**Contexto**: el MVP debe entregar plugins reales con UI declarativa, permisos y
aislamiento. Integrar Wasmtime + Component Model en el mismo ciclo de entrega
añade una dependencia pesada y retrasa el editor/lienzo, que son el núcleo del
producto.

**Decisión**: congelar primero el contrato (`Manifest`, `Request`, `Response`,
`UiNode`, permisos, límites, firma Ed25519) y ejecutar los cinco plugins de
referencia con un runtime Rust que aplica exactamente las mismas validaciones y
permisos. El paquete y el WIT están publicados; instalar/desinstalar, habilitar y
revocar permisos funcionan. El cargador Wasmtime/WIT se añade en 1.0 detrás del
trait `PluginRuntime`, sin cambiar el contrato ni el estado de los bloques.

**Consecuencias**:
- El MVP demuestra UI declarativa, permisos y aislamiento de panic sin el coste de
  Wasmtime; los plugins de terceros aún no ejecutan binarios.
- El estado del bloque vive en la nota y sobrevive a desinstalar/reinstalar.
- La API se puede congelar antes de añadir el runtime, reduciendo el riesgo de
  deriva.
- La puerta E10 se cumple parcialmente: los 3 plugins MVP funcionan, pero el
  sandbox WASM de terceros es 1.0.

**Alternativas**: Wasmtime en el MVP (mejor fidelidad, más riesgo de calendario);
ABI JSON con wasmtime core (Plan B del roadmap); no tener plugins (rechazado).

---

## ADR-024 — Undo por instantáneas acotadas

**Fecha**: 2026-10-29
**Estado**: aceptada

**Contexto**: el undo debe abarcar texto, formato, canvas y DnD con una sola pila.
Las operaciones de texto y los cambios de modelo compiten por el mismo historial.

**Decisión**: `UndoStack` guarda hasta 100 instantáneas del `Note`. Cada comando
revisa el modelo completo, así que undo/redo es uniforme y sencillo. La pila se
limpia al cambiar de nota. El autosave se desacopla del undo.

**Consecuencias**:
- Implementación pequeña y robusta; cumple 100 pasos y undo de formato/canvas.
- La memoria crece con el tamaño de la nota × 100; en notas de 50 MB puede ser
  alto. En 1.0 se puede pasar a deltas por bloque con la misma interfaz.
- La recuperación tras crash usa snapshots de `doctor::write_recovery`.

**Alternativas**: deltas por comando (más eficiente, más complejo y fácil de
romper); undo nativo de GTK (no cubre canvas).

---

## ADR-025 — Configuración escrita con `toml_edit`

**Fecha**: 2026-10-29
**Estado**: aceptada

**Contexto**: la GUI y la CLI deben producir el mismo archivo, preservando
comentarios, orden y claves desconocidas.

**Decisión**: un único esquema (`folio_config::schema`) genera validación, GUI,
JSON Schema y ayuda. Las escrituras usan `toml_edit`, conservan decoraciones y
claves desconocidas, y validan recargando antes de dar el cambio por bueno. La
precedencia es CLI > entorno > bóveda > global > defectos.

**Consecuencias**:
- Paridad comprobada por tests y por el mismo `write_value`.
- Un TOML inválido no se aplica; se reporta archivo/línea y se conserva la última
  configuración válida en memoria.
- Los secretos nunca van al TOML.

**Alternativas**: reescribir el archivo desde serde (pierde comentarios);
configuración en SQLite (opaca, rechazada).

---

## ADR-026 — UI GTK construida a mano con un esquema común

**Fecha**: 2026-10-29
**Estado**: aceptada

**Contexto**: el MVP necesita una UI nativa sin generar código frágil ni añadir un
framework de UI sobre GTK.

**Decisión**: `folio-ui` compone widgets GTK/Libadwaita directamente. El editor
usa un `GtkTextView` por bloque con tags; los bloques se reordenan y anclan en un
`GtkFixed` para el lienzo. Los diálogos de ajustes y paleta se generan desde el
esquema y los comandos. Las operaciones de disco se hacen fuera del hilo de
pintado cuando son costosas; el guardado es atómico y con debounce.

**Consecuencias**:
- Menos capas y mejor integración con accesibilidad/IME nativos.
- El código de UI es explícito y verificable con smoke tests en Xvfb.
- El zoom de texto del lienzo y la edición a bajo zoom se posponen a 1.0.
- La UI no habla con SQLite directamente: usa `Store` e `Index` a través de
  `AppState`.

**Alternativas**: motor propio de texto (semanas de trabajo); WebView (rechazado
por RAM y seguridad); framework declarativo externo (más dependencias).

---

## ADR-027 — Vista web viva efímera y opt-in

**Fecha**: 2026-10-29
**Estado**: aceptada para 1.0

**Contexto**: la vista web viva amplía la superficie de ataque: cookies, storage,
descargas, permisos, procesos de render y contenido remoto. Debe ser opcional y no
contaminar la bóveda ni la sesión.

**Decisión**: compilar la vista viva solo con `--features webkit` (WebKitGTK 6.0).
Cada bloque usa una `NetworkSession` efímera, cookie policy `Never`, sin
almacenamiento local/base de datos/caché de página/WebGL, sin extras de desarrollo,
sin acceso a `file://`, permisos denegados, ventanas emergentes bloqueadas y
descargas canceladas. La carga exige confirmación explícita. Al desmontar el widget
se llama a `terminate_web_process`; también hay un botón «Descargar contenido y
cerrar». La URL se valida con el mismo esquema que las tarjetas (`http/https`).

**Consecuencias**:
- El binario base no depende de WebKit; Flatpak/AUR pueden ofrecer la feature.
- Los tests de aislamiento se ejecutan en Xvfb con `scripts/webkit.sh`.
- No hay cookies persistentes ni caché reutilizable entre bloques.
- La feature no se incluye por defecto para no romper CI sin WebKit 6.0.

**Alternativas**: WebView del sistema (no disponible en GTK4 de forma estable);
vista viva por defecto (rechazada por seguridad y RAM); descargar HTML y renderizar
con WebKit oculto (más complejo y sin interacción).

---

## ADR-028 — Cifrado de bóveda con clave envuelta e índice efímero

**Fecha**: 2026-10-29
**Estado**: aceptada para 1.0

**Contexto**: el cifrado debe ser opcional, no filtrar títulos ni contenido en
archivos temporales, y permitir recuperación. SQLCipher añade complejidad de
empaquetado; un índice SQLite en claro filtraría todo el texto.

**Decisión**: VK aleatoria de 32 bytes; `KEK = Argon2id(password, salt, m=64 MiB,
t=3, p=4)`; VK envuelta con XChaCha20-Poly1305 en `.folio/keyring.json` (0600).
Cada archivo de datos se cifra con XChaCha20-Poly1305 y nonce aleatorio en formato
`FOLIOENC1 || nonce || ciphertext`; `vault.toml` y el keyring quedan en claro
(metadatos no secretos). En bóvedas cifradas el índice vive solo en memoria y se
reconstruye al desbloquear. La frase de recuperación BIP39 de 24 palabras codifica
la VK y se verifica contra un hash guardado en el keyring. La GUI descarta la clave
y los buffers al bloquear; la caché de assets descifrados vive en
`.folio/cache/plain` y se borra al bloquear. La CLI exige `--password`,
`FOLIO_PASSWORD` o prompt.

**Consecuencias**:
- `strings` no encuentra títulos, cuerpo ni adjuntos en los archivos fuente.
- Los temporales de escritura atómica se crean ya cifrados.
- La búsqueda en bóvedas cifradas paga una reconstrucción de índice por proceso
  CLI y una al desbloquear en GUI; es el precio de no persistir texto claro.
- Los nombres de carpetas nuevas son opacos; una bóveda existente conserva sus
  carpetas al cifrarse.
- La recuperación no depende del keyring para descifrar, solo para verificar que
  la frase pertenece a la bóveda.

**Alternativas**: SQLCipher (más dependencias y claves en el proceso); cifrar solo
notas (filtra book.toml y assets); no persistir assets descifrados (requiere
GdkTexture/GStreamer en memoria; se puede refinar en 1.0).

---

## ADR-029 — PDF con Cairo/Pango y lienzo paginado

**Fecha**: 2026-10-29
**Estado**: aceptada para 1.0

**Contexto**: el PDF debe sobrevivir a la app, incluir tablas, imágenes y la
posición visual de los bloques anclados, y ser verificable en CI sin depender de
un navegador.

**Decisión**: `folio-render::pdf` usa `cairo::PdfSurface` y PangoCairo. A4
(595×842 pt), márgenes de 48 pt, paginación por bloques y párrafos, tablas con
saltos de página, imágenes resueltas por un callback (`AssetResolver`) que el
llamador conecta con la bóveda (y por tanto descifra si hace falta). Las tarjetas
web y los plugins se dibujan como tarjetas estáticas; nunca se renderiza la vista
web viva. Los bloques `free` van a una página «Lienzo» escalada a su bbox, con sus
coordenadas. Metadatos de fecha fijos para golden determinista.

**Consecuencias**:
- Misma fuente de datos para MD/HTML/PDF; sin WebKit en el exportador.
- Las imágenes se leen del archivo resuelto, por lo que funciona con bóvedas
  cifradas usando la caché descifrada temporal.
- El golden compara texto extraído con `pdftotext` normalizado, no bytes, para no
  depender de la versión de fuentes/Cairo.
- Los plugins se representan como tarjeta de estado, no como captura interactiva.

**Alternativas**: WebKit print-to-PDF (dependencia y no determinista); Cairo a
mano sin Pango (peor texto); PDF en 1.1 (rechazado).

---

## ADR-030 — Auditoría de accesibilidad y alternativa al lienzo

**Fecha**: 2026-10-29
**Estado**: aceptada para 1.0

**Contexto**: el lienzo puede ser inaccesible para teclado y lectores de pantalla;
los botones de icono no siempre tienen nombre; y la escala 200 % puede romper
layouts.

**Decisión**: auditoría automatizada en `scripts/a11y.sh` (Xvfb, escala 200 %,
puente a11y), etiquetas `Property::Label` en todos los botones de icono, foco
visible por CSS, título enfocado al abrir y una **lista de bloques** accesible
(`Ctrl+Shift+B`) con acciones de subir/bajar/anclar/editar/eliminar. `Esc` sale del
lienzo al documento. El tema claro/oscuro se prueba en vivo. Las animaciones se
mantienen mínimas y se respeta `ui.reduce_motion` en el diseño.

**Consecuencias**:
- El canvas tiene una alternativa completa por teclado.
- La auditoría automática cubre estructura y escala, pero no sustituye la revisión
  con Orca/lector real, que queda como checklist de release.
- Los botones internos de Libadwaita reciben una etiqueta de respaldo.

**Alternativas**: canvas solo con ratón (rechazado); vista de lista como única
opción (limita el producto); auditoría manual únicamente (no repetible).

---

## ADR-031 — Paquete .deb verificable sin privilegios

**Fecha**: 2026-10-29
**Estado**: aceptada para 1.0

**Contexto**: Arch/AUR son el entorno de desarrollo, pero 1.0 exige un `.deb`
instalable; en el entorno de desarrollo no hay `dpkg-deb` ni root.

**Decisión**: `scripts/build-deb.sh` crea el árbol FHS y produce el paquete con
`dpkg-deb` si existe; en su defecto usa `scripts/deb_build.py` (formato ar con
`debian-binary`, `control.tar.gz` y `data.tar.gz`). La verificación extrae el
paquete en un directorio temporal, comprueba archivos/permisos, ejecuta
`folio --version` y valida AppStream. El paquete incluye binarios, desktop,
metainfo, icono, manual, recuperación y copyright.

**Consecuencias**:
- El `.deb` se prueba en CI y en Arch sin privilegios.
- Si `dpkg-deb` está disponible se usa la herramienta nativa.
- La instalación real en una VM limpia sigue siendo una verificación manual.

**Alternativas**: solo Flatpak (rechazado para 1.0); `cargo-deb` (dependencia extra
y menos control del contenido); no empaquetar (rechazado).

---

## ADR-032 — Canal beta local, sin telemetría y con rollback probado

**Fecha**: 2026-10-29
**Estado**: aceptada para 1.0

**Contexto**: la beta debe recoger feedback real durante 30 días sin introducir
telemetría ni dependencias de red, y debe poder revertirse sin tocar datos.

**Decisión**: `scripts/beta.sh` prepara un canal local en `dist/beta/<versión>`
con `.deb`, binarios, checksums, informe de pre-beta y `CHANNEL.json` que declara
`telemetry: false`. `scripts/beta-check.sh` ejecuta un escenario completo (bóveda,
búsqueda, campos desconocidos, backup/restore, MD/HTML/PDF, cifrado y ausencia de
clientes de telemetría/red). El feedback es manual, por issue/email. Los P0/P1 y
el protocolo de rollback están en `docs/20-beta-30-dias.md`.

**Consecuencias**:
- La beta no envía datos ni depende de un servicio de analítica.
- El canal se puede reproducir offline y verificar con `SHA256SUMS`.
- La release 1.0 no se bloquea por telemetría; solo por P0/P1, migraciones y
  validación en máquina limpia.

**Alternativas**: telemetría opt-in (aún requiere infraestructura y confianza);
crash reporting remoto (rechazado); beta solo interna (no valida uso real).

---

## ADR-033 — Runtime WASM de terceros: ABI JSON sobre wasmi

**Fecha**: 2026-10-29
**Estado**: aceptada para 1.0

**Contexto**: el contrato de UI y el ciclo de vida ya estaban congelados, pero
faltaba ejecutar módulos de terceros. Wasmtime/Component Model añade peso y
complejidad de empaquetado; el plan permitía un ABI JSON como Plan B.

**Decisión**: añadir `folio_plugins::wasm::WasmRuntime` sobre **wasmi** (intérprete
puro Rust, sin WASI). El módulo exporta `folio_api_version`, `folio_alloc`,
`folio_handle`, `folio_result_ptr`, `folio_result_len` y `memory`; la petición y
la respuesta son JSON con el contrato `api = 1`. Cada llamada crea Store e
instancia nuevos, limita memoria (64 MB por defecto), fuel, tamaño de estado y
valida la UI. Las funciones de host consultan `HostContext`; sin permiso de red no
se concede. Los plugins de referencia siguen usando el runtime Rust equivalente.

**Consecuencias**:
- Plugins de terceros funcionan sin WASI ni acceso ambiental.
- El binario base no incluye Wasmtime; wasmi es una dependencia pequeña.
- El WIT sigue documentado como contrato de migración futura.
- Un plugin que trapa o agota fuel se convierte en error aislado; la app sigue.

**Alternativas**: Wasmtime+WIT (más fidelidad, más peso); solo builtins (rechazado
para 1.0); ABI binario propietario (peor portabilidad).

---

## Plantilla para nuevas ADR

```markdown
## ADR-0XX — Título

**Fecha**: YYYY-MM-DD
**Estado**: propuesta | aceptada | reemplazada por ADR-0YY

**Contexto**: …

**Decisión**: …

**Consecuencias**: …

**Alternativas**: …
```
