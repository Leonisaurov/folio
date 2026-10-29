# 01 — Visión, alcance y glosario

## 1. Visión

> **Folio convierte el cuaderno diario en un espacio de trabajo infinito: escribir debe sentirse tan natural como en una hoja, pero nada debería impedir mover una idea, un dibujo o una gráfica al lugar donde pertenece.**

Folio no es «otra app de notas con Markdown». Es un lugar donde el libro del día aparece sin fricción, la escritura fluye en una columna cómoda y, cuando hace falta, la página se expande en un lienzo donde cada elemento tiene posición, tamaño y vida propia. El usuario no debería pensar en «bloques» ni en «formatos»: debería pensar en sus ideas.

La aplicación es **local-first**, **portátil** y **segura por defecto**. El contenido es del usuario, no de un servicio. La app debe seguir siendo útil dentro de diez años, incluso si el proyecto desaparece: los archivos se entienden sin ella y la exportación es siempre posible.

## 2. Usuario y contexto

### Persona principal

- Una persona que usa Linux a diario (GNOME/Wayland), escribe mucho y valora rapidez, teclado y privacidad.
- Quiere un cuaderno diario automático, pero también libros temáticos creados a mano.
- Necesita mezclar texto, imágenes, tablas, dibujos, audio, vídeo y referencias web en una misma nota.
- Le interesa extender la app con pequeños plugins propios sin depender de un marketplace.
- No quiere Electron, ni suscripciones, ni que sus notas salgan del disco sin permiso.

### Trabajos por hacer (JTBD)

| Cuando… | Quiero… | Para… |
|---|---|---|
| Empiezo el día | Que exista un libro de hoy sin crear nada | Capturar ideas de inmediato |
| Estoy escribiendo | Que el texto fluya como un documento | No pelear con el formato |
| Necesito ordenar visualmente | Mover bloques libremente en un lienzo | Pensar en el espacio, no solo en líneas |
| Termino una nota | Reordenarla o moverla a otro libro | Mantener el sistema vivo |
| Vuelvo a un tema | Encontrarlo por texto, libro o fecha | No perder tiempo |
| Quiero una vista específica | Añadir una gráfica, barra o diagrama | Ver mis datos, no solo leerlos |
| Comparto o archivo | Exportar a PDF/HTML/Markdown | Que el contenido salga conmigo |
| Me preocupo por privacidad | Cifrar una bóveda y controlar plugins | Dormir tranquilo |

## 3. Principios de experiencia

1. **Cero fricción al abrir**: el libro de hoy está a un clic; si no existe, se crea solo.
2. **La escritura manda**: la columna de texto es el estado por defecto; el lienzo es una capacidad, no un peaje.
3. **Libertad reversible**: anclar, mover y recomponer siempre se puede deshacer.
4. **El teclado no es ciudadano de segunda**: toda acción tiene atajo o alternativa accesible.
5. **Lo importante se ve**: título, libro, fecha y estado de guardado siempre claros.
6. **Nada de modales innecesarios**: preferir popovers, toasts con «Deshacer» y edición en línea.
7. **La app desaparece**: sin distracciones, sin métricas, sin cuentas.
8. **Rendimiento percibido**: ninguna acción frecuente debería mostrar un spinner.

## 4. Glosario

| Término | Significado preciso en Folio |
|---|---|
| **Bóveda** | Carpeta raíz autocontenida con libros, notas, adjuntos, índice y configuración. Es la unidad de apertura, backup y cifrado. |
| **Libro** | Contenedor de notas. Puede ser diario (ligado a una fecha lógica) o manual. Se puede renombrar, archivar y eliminar. |
| **Nota** | Documento dentro de un libro. Tiene título, bloques, layout y metadatos. |
| **Bloque** | Unidad de contenido de una nota: texto, tabla, imagen, audio, vídeo, archivo, web, dibujo o plugin. |
| **Flujo** | Layout lineal por defecto: los bloques se apilan en una columna legible. |
| **Anclado / libre** | Bloque con posición explícita en el lienzo; deja de participar en el flujo. |
| **Lienzo** | Espacio infinito de coordenadas donde viven el flujo y los bloques anclados. |
| **Recomponer** | Acción que ordena los bloques por posición visual, limpia anclajes y vuelve al flujo. Es deshacible. |
| **Encuadre documento** | Vista que centra la columna de flujo y atenúa los bloques anclados. |
| **Orden manual / libre** | Orden de notas definido por el usuario; se conserva aparte de los órdenes automáticos. |
| **Orden de nota** | Clave fraccional (`order_key`) que determina la posición dentro del libro; se refleja en el nombre del archivo. |
| **Spring-load** | Abrir automáticamente un libro tras mantener el puntero sobre él durante un tiempo al arrastrar una nota. |
| **Plugin** | Paquete WebAssembly con manifiesto, permisos y UI declarativa. |
| **Capacidad** | Permiso concreto que el host concede a un plugin (`note.read`, `storage`, `network`…). |
| **Bóveda cifrada** | Bóveda cuyo contenido y metadatos están cifrados con una clave derivada de contraseña. |
| **Día lógico** | Fecha a la que pertenece una nota, calculada con zona horaria y hora de corte configurables. |
| **Bloque de plugin** | Bloque cuyo estado y render pertenecen a un plugin; el host conserva el JSON aunque falte el plugin. |

## 5. Alcance por prioridad

### 5.1 Must — sin esto no hay MVP

| ID | Capacidad | Criterio de aceptación |
|---|---|---|
| M-01 | Bóveda local con archivos legibles | Crear, abrir, cerrar y reabrir sin pérdida; el índice se puede borrar y reconstruir |
| M-02 | Libro diario automático configurable | Al abrir el día lógico aparece el libro; título/carpeta/hora/zona configurables; renombrar no duplica |
| M-03 | Libros manuales | Crear, renombrar, archivar, borrar y restaurar |
| M-04 | Notas ilimitadas | 10k notas en una bóveda sin degradación inaceptable |
| M-05 | Orden manual y DnD | Mover dentro y entre libros; spring-load; selector de posición; undo |
| M-06 | Órdenes alternativos | Título, creación, modificación; no destruyen el orden manual |
| M-07 | Editor de texto rico | Párrafos, H1–H3, negrita/cursiva/subrayado/tachado/código, listas, checklist, cita, código, enlaces |
| M-08 | Lienzo híbrido | Flujo por defecto, anclaje por arrastre, recomposición, zoom, paneo, encuadre |
| M-09 | Tablas básicas | Crear, editar celdas, filas/columnas, pegar CSV, exportar |
| M-10 | Medios | Imagen, audio, vídeo, archivo; copia local; reproducción; límites |
| M-11 | Web tarjeta | Tarjeta de enlace; sin red por defecto; vista viva en 1.0 |
| M-12 | Dibujo básico | Trazo con ratón o stylus si reporta presión, borrador, deshacer, exportar SVG/PNG |
| M-13 | Previsualización | Solo lectura, medios y plugins en modo lectura |
| M-14 | Búsqueda | FTS5 con filtros y resaltado; < 80 ms en 10k notas |
| M-15 | Plugins | Host, SDK, permisos, UI nativa, gestor y 3 ejemplos (progreso, gráficas, diagramas) |
| M-16 | Configuración texto + GUI + CLI | Paridad de claves; TOML comentado; recarga en vivo |
| M-17 | Exportación/importación | Markdown y HTML; importar Markdown; PDF en 1.0 |
| M-18 | Integridad | Escritura atómica, papelera, versiones, backups, recuperación tras `kill -9` |
| M-19 | Seguridad | Sandbox de plugins, rutas seguras, dependencias auditadas; web viva aislada en 1.0 |
| M-20 | Entrega Linux | Flatpak y AUR; AppStream; `.deb` en 1.0 |

### 5.2 1.0 — comprometido después del MVP

Estas capacidades son parte del producto y están planificadas, pero no bloquean el MVP de 14 semanas:

- Bóveda cifrada, bloqueo automático y clave de recuperación.
- Vista web viva aislada y bajo demanda.
- Exportación PDF con fidelidad de canvas.
- Plugins `quickbar` (barra de acciones) y `stats`, hasta completar los 5 ejemplos.
- `.deb` y auditoría de accesibilidad.
- SDK de plugins 1.0 congelado y documentado.
- Cierre de la beta de 30 días iniciada en el MVP y release estable.

### 5.3 Should — si el calendario del MVP lo permite

- Grabación de audio dentro de la nota.
- Plantillas de nota y de libro diario.
- Etiquetas y favoritos.
- Historial de versiones visible en UI.
- Atajos configurables.
- Búsqueda de texto en adjuntos PDF (sin OCR).

### 5.4 Could — después de 1.0

- Ventanas múltiples o pestañas.
- Vistas guardadas y filtros complejos.
- Enlaces entre notas y backlinks.
- Calendario/heatmap de actividad.
- Importadores de Obsidian/Notion/Evernote.
- Temas personalizados por tokens.
- Grabación de pantalla.
- Plugin de IA local opcional.

### 5.5 Won't — no se hace

- Sincronización en tiempo real en el MVP.
- Colaboración multiusuario.
- Versiones móvil, Windows o macOS.
- DOCX de ida y vuelta.
- OCR, reconocimiento de escritura o transcripción.
- Marketplace público de plugins.
- Ejecución de JavaScript de plugins fuera del WebView aislado.
- Telemetría obligatoria.
- Cuentas o nube propietaria.

## 6. Escenario de aceptación del MVP

Este escenario es la definición operativa de «terminado». Se ejecuta manualmente al final de cada etapa afectada y completo antes de la beta.

1. **Instalación limpia** en Arch/GNOME con Flatpak.
2. **Bóveda**: crear `~/Notas`, elegir tema oscuro, activar libros diarios.
3. **Libro de hoy**: aparece `29/10/2026`; renombrarlo a `Jueves`; cerrar y abrir: no se crea otro libro del día.
4. **Nota rica**: título, dos párrafos, negrita, lista, checklist, tabla 3×3, imagen arrastrada, dibujo con 20 trazos y una tarjeta web con fallback offline.
5. **Lienzo**: arrastrar la tabla fuera del flujo; ver el hueco cerrarse; `Ctrl+1` enfoca el documento; «Recomponer» devuelve la tabla al flujo; deshacer dos veces.
6. **Orden**: crear libro «Proyectos»; arrastrar la nota sobre él; a los 650 ms se abre; elegir posición 2; soltar; comprobar que el libro de hoy ya no la tiene y que «Proyectos» la muestra en la posición elegida.
7. **Configuración**: cambiar `daily.title_format` a `%Y/%m/%d` en la GUI y `daily.folder_format` a `%d-%m-%Y` en `vault.toml`; ver el cambio sin reiniciar; provocar un error TOML y comprobar que se conserva la última configuración válida.
8. **Plugins**: insertar barra de progreso, gráfica vinculada a la tabla y diagrama con DSL; interactuar con los tres.
9. **Búsqueda**: buscar una palabra; abrir el resultado con el fragmento resaltado.
10. **Exportación**: HTML y Markdown; abrir ambos y comprobar contenido, medios y canvas (PDF en 1.0).
11. **Caos**: `kill -9` durante una edición; reabrir; no hay pérdida ni nota corrupta; la recuperación ofrece la versión guardada.
12. **Rendimiento**: arranque < 600 ms; RAM < 150 MB con la bóveda de prueba; búsqueda < 80 ms.

**Escenario 1.0** (además de todo lo anterior): cifrar la bóveda y desbloquearla; cargar una vista web viva y comprobar que no persiste cookies; exportar PDF con canvas; usar `quickbar` y `stats`; instalar el `.deb`; pasar la auditoría de accesibilidad.

## 7. Métricas de éxito

| Métrica | Objetivo | Cómo se mide |
|---|---|---|
| Retención personal | 14 días consecutivos de uso | Diario de uso |
| Pérdida de datos | 0 casos | Pruebas de caos + reportes |
| Crash-free | ≥ 99 % de sesiones | Beta, logs locales opt-in |
| Arranque | < 600 ms p95 | Benchmark CI |
| RAM | < 150 MB con 10k notas | Benchmark CI |
| Búsqueda | < 80 ms | Benchmark CI |
| Plugins de ejemplo | 5 funcionando en CI | Arnés de conformidad |
| Accesibilidad | 100 % acciones por teclado | Checklist manual |
| Documentación de plugin | Un tercero crea uno siguiendo la guía | Prueba con usuario |

## 8. Supuestos

- El usuario trabaja en una sola máquina durante el MVP; la sincronización no es necesaria para validar el producto.
- La bóveda vive en un sistema de archivos local con soporte de `rename` atómico y `fsync` (ext4, btrfs, xfs).
- GNOME/Wayland es el entorno principal; X11 funciona, pero no se optimiza.
- Hay conexión a internet solo para descargar dependencias y, opcionalmente, metadatos de enlaces; la app funciona offline.
- El usuario acepta que el nombre «Folio» es provisional y que la licencia se confirmará antes de publicar.
- El desarrollo es de una sola persona; las estimaciones incluyen documentación y pruebas, no solo código.

## 9. Fuera de alcance y por qué

| Excluido | Razón |
|---|---|
| Sync en MVP | Duplica la complejidad de conflictos y identidad; primero hay que estabilizar el modelo local. |
| Colaboración | Requiere CRDT, presencia, permisos y servidor; producto distinto. |
| Móvil | El objetivo es una app Linux nativa excelente, no multiplataforma. |
| DOCX | Fidelidad baja y mantenimiento alto; Markdown/HTML/PDF cubren el intercambio. |
| OCR/IA | Coste de modelos, privacidad y alcance; se puede añadir como plugin local después. |
| Marketplace | La seguridad y la curaduría requieren un equipo; los plugins se instalan desde archivo. |

## 10. Relación con el resto del plan

- La arquitectura que hace posible este alcance: [02-arquitectura.md](02-arquitectura.md).
- Los datos que garantizan portabilidad: [03-modelo-datos.md](03-modelo-datos.md).
- La experiencia concreta: [04-experiencia-ui.md](04-experiencia-ui.md).
- La configuración completa: [05-configuracion.md](05-configuracion.md).
- Los plugins: [06-plugins.md](06-plugins.md).
- La seguridad y el rendimiento: [07-seguridad-rendimiento.md](07-seguridad-rendimiento.md).
- La ejecución semana a semana: [09-hoja-de-ruta.md](09-hoja-de-ruta.md).
