# 11 — Preguntas, decisiones pendientes y supuestos

## 1. Preguntas ya cerradas

Se hicieron dos rondas de preguntas antes de escribir este plan. Estas son las respuestas que hoy mandan:

| # | Pregunta | Respuesta | Dónde se aplica |
|---|---|---|---|
| 1 | Stack | Rust + GTK4/Libadwaita | ADR-001, E0 |
| 2 | Modelo de nota | Lienzo infinito con flujo por defecto y anclaje libre | ADR-003, E5 |
| 3 | Almacenamiento | Archivos + índice SQLite | ADR-002, E1 |
| 4 | Plugins | WASM + UI nativa declarativa | ADR-009, E10 |
| 5 | Sync | Local primero, después | ADR-013 |
| 6 | Cifrado | Bóveda cifrada opcional | ADR-012, E11 |
| 7 | Plazo | Solo el usuario, MVP 10–14 semanas | E0–E13 |
| 8 | Libros diarios | Creación perezosa + relleno manual | ADR-007, E3 |
| 9 | Web/medios | Tarjeta + vista viva opcional; medios locales | ADR-011, E7 |
| 10 | Exportación | Markdown, HTML, PDF; importar Markdown | E9 |
| 11 | Idioma | Español primero, i18n preparado | ADR-017 |
| 12 | Sync futura | Decidir después | Etapa 14 (post-MVP) |

## 2. Preguntas abiertas

Ninguna bloquea el arranque. Cada una tiene un **defecto** y una **fecha límite**; si no se responde, se aplica el defecto y se registra en la ADR correspondiente.

| # | Pregunta | Defecto si no hay respuesta | Límite | Impacto |
|---|---|---|---|---|
| Q-01 | ¿Nombre definitivo y licencia? | «Folio», GPL-3.0-or-later + SDK MIT/Apache-2.0 | Antes de E13 | Marca, AppStream, repo |
| Q-02 | ¿Libros anidados/carpetas? | No en MVP; agrupar por año/mes | Antes de E2 | Navegación |
| Q-03 | ¿Una nota puede estar en varios libros? | No; mover, no duplicar | Antes de E6 | Modelo de datos |
| Q-04 | ¿Canvas por nota o por libro? | Por nota | Antes de E5 | Layout |
| Q-05 | ¿Título de nota obligatorio? | No; derivar de la primera línea | Antes de E4 | UI |
| Q-06 | ¿Límite de tamaño de nota? | 50 MB JSON, aviso >10 MB; medios fuera | Antes de E4 | Rendimiento |
| Q-07 | ¿Plantillas de libro completas? | Solo plantilla de nota simple en MVP | Antes de E3 | Alcance |
| Q-08 | ¿Backlinks/grafo? | Post-1.0 | Después de 1.0 | Alcance |
| Q-09 | ¿Recordatorios/calendario? | Post-1.0 | Después de 1.0 | Alcance |
| Q-10 | ¿IA local? | Post-1.0, opt-in, como plugin | Después de 1.0 | Privacidad |
| Q-11 | ¿DOCX? | Post-1.0 | Después de 1.0 | Exportación |
| Q-12 | ¿Marketplace de plugins? | Post-1.0; instalar desde archivo | Después de 1.0 | Seguridad |
| Q-13 | ¿Backend de sync? | Decidir en semana 14; WebDAV/S3 + CRDT como candidato | Semana 14 | Arquitectura futura |
| Q-14 | ¿Temas CSS personalizados? | Solo tokens en MVP; CSS en 1.1 | Después de 1.0 | Seguridad |
| Q-15 | ¿Imprimir directamente? | HTML/Markdown en MVP; PDF e impresión en 1.0 | Antes de E9 | Alcance |
| Q-16 | ¿Captura de páginas web? | Post-1.0 | Después de 1.0 | Alcance |
| Q-17 | ¿OCR/escritura a mano? | Post-1.0 | Después de 1.0 | Alcance |
| Q-18 | ¿Colaboración? | Post-1.0 | Después de 1.0 | Producto |
| Q-19 | ¿Windows/macOS? | No; solo Linux | — | Producto |
| Q-20 | ¿Móvil? | No; solo Linux | — | Producto |
| Q-21 | ¿Flatpak, AUR y deb? | Flatpak y AUR en MVP; `.deb` en 1.0 | Antes de E13 | Distribución |
| Q-22 | ¿Distros mínimas? | Arch, Fedora 40+, Ubuntu 24.04+; Flatpak cubre el resto | Antes de E0 | Dependencias |
| Q-23 | ¿Idioma inglés en MVP? | No; infraestructura lista | Antes de E12 | i18n |
| Q-24 | ¿Telemetría? | Ninguna | — | Privacidad |
| Q-25 | ¿Nombre del binario? | `folio` | Antes de E0 | CLI, paquetes |
| Q-26 | ¿Aceptas que cifrado, web viva, PDF, `quickbar`/`stats` y `.deb` pasen a 1.0 para mantener el MVP en 14 semanas? | Sí; MVP 14 semanas y 1.0 en 20. Alternativa: extender el MVP a 20 semanas o recortar otras funciones. | Antes de E0 | Calendario y alcance |

## 3. Cómo se resuelven

1. El usuario responde en la conversación o en una issue `decision`.
2. Se crea/actualiza una ADR en [10-decisiones.md](10-decisiones.md).
3. Si cambia el alcance, se actualiza [09-hoja-de-ruta.md](09-hoja-de-ruta.md) y [14-trazabilidad.md](14-trazabilidad.md).
4. Si la decisión afecta datos, se añade migración y prueba.
5. Si no hay respuesta en la fecha límite, se aplica el defecto y se anota «decisión por defecto» en la ADR.

## 4. Supuestos explícitos

- El usuario es la única persona que escribe en la bóveda durante el MVP.
- La bóveda está en un sistema de archivos local con `rename` atómico y `fsync` (ext4, btrfs, xfs).
- GNOME/Wayland es el entorno principal; X11 funciona.
- Hay conexión solo para descargar dependencias y, opcionalmente, metadatos web.
- El hardware de referencia es un portátil de 4 núcleos y 8 GB.
- El usuario acepta que el MVP no tiene sync ni colaboración.
- El nombre «Folio» es provisional.
- La licencia se confirma antes de publicar.
- Los plugins se instalan desde archivo; no hay marketplace en MVP.
- La app no envía datos a ningún servicio.

## 5. Preguntas que NO se harán

Para evitar bloqueos innecesarios, se decide por defecto:

- ¿Qué fuente exacta? La del sistema; configurable.
- ¿Qué icono? Set simbólico de Libadwaita; identidad final en E12.
- ¿Qué animación? 150 ms ease-out; respeta movimiento reducido.
- ¿Qué color de acento? Azul Libadwaita; configurable.
- ¿Qué formato de ID? UUIDv7.
- ¿Qué algoritmo de hash? BLAKE3.
- ¿Qué compresión de versiones? zstd.
- ¿Qué base de datos? SQLite con FTS5.
- ¿Qué runtime de plugins? Wasmtime.
- ¿Qué formato de config? TOML.

Si alguna de estas elecciones demuestra ser inviable en un spike, se cambia con ADR y se actualiza el plan; no se abre una pregunta al usuario salvo que afecte al producto.
