# 04 — Experiencia, interfaz e interacción

## 1. Principios de diseño

- **Una sola ventana, tres zonas**: barra lateral (dónde estoy), lista de notas (qué hay), editor (en qué trabajo). Se pueden colapsar para concentrarse.
- **El libro de hoy siempre a un clic**, sin buscarlo.
- **La fecha es contexto, no burocracia**: el título del libro diario se puede renombrar, pero la fecha queda como insignia.
- **El lienzo no secuestra la escritura**: se abre en modo documento; el espacio libre se revela al arrastrar o al hacer zoom-out.
- **Todo se deshace**: mover, anclar, recomponer, borrar y mover entre libros.
- **Nada modal para lo frecuente**: renombrar en línea, crear con `+`, elegir con popovers.
- **Estados vacíos que enseñan**: una frase y una acción, no un muro de texto.
- **Estética sobria y cálida**: papel claro, tinta oscura, acento azul; modo oscuro cuidado; sombras suaves; nada de gradientes llamativos.

## 2. Arquitectura de información

```text
Ventana
├── Barra lateral
│   ├── Selector de bóveda
│   ├── Buscar (global)
│   ├── Hoy  ← libro diario actual (o crear)
│   ├── Libros
│   │   ├── Fijados
│   │   ├── 2026
│   │   │   ├── Octubre
│   │   │   │   ├── 29/10/2026
│   │   │   │   └── Proyectos
│   │   │   └── …
│   │   └── Archivados
│   ├── Etiquetas (1.1)
│   ├── Papelera
│   └── Plugins
├── Lista de notas
│   ├── Cabecera del libro + contador
│   ├── Orden: Manual / Título / Creación / Modificación
│   ├── Buscar en el libro
│   └── Notas (filas con título, extracto, fecha, iconos de medios)
└── Editor
    ├── Barra superior: libro / título / estado de guardado / acciones
    ├── Lienzo o documento
    ├── Barra de formato contextual
    └── Panel derecho opcional: esquema, propiedades, plugin
```

## 3. Primer arranque

1. Ventana de bienvenida con nombre «Folio» y una ilustración discreta.
2. **Crear bóveda** (recomendado) o **Abrir bóveda**.
3. Elegir carpeta con el selector del sistema (portal en Flatpak). Nombre de la bóveda.
4. Interruptor «Crear un libro cada día» (activado). Formato por defecto `29/10/2026`.
5. Interruptor «Cifrar esta bóveda» (desactivado). Si se activa: contraseña, confirmación, medidor de fuerza, aviso de que no se puede recuperar sin la clave, generación de clave de recuperación.
6. Tema: sistema/claro/oscuro.
7. «Empezar». Se crea el libro de hoy con una nota de bienvenida que explica los tres gestos básicos: escribir, arrastrar un bloque, buscar.

Duración objetivo: < 60 segundos. Se puede omitir todo y cambiar después.

## 4. Ventana principal

### 4.1 Barra lateral

- Ancho 240–320 px, colapsable con `F9` o botón.
- Cabecera: nombre de bóveda + menú (cambiar, ajustes, cerrar).
- Búsqueda global con `Ctrl+K` (paleta) o campo dedicado.
- **Hoy**: si existe, muestra el título (aunque se haya renombrado) y la fecha; si no, «Crear libro de hoy».
- Árbol de libros agrupado por año/mes; los fijados arriba; archivados al final.
- Cada libro muestra icono, título, número de notas y un punto si tiene actividad reciente.
- Arrastrar una nota sobre un libro: resaltado; a los 650 ms se abre (spring-load) y se muestran las posiciones.
- Menú contextual de libro: renombrar, fijar, icono/color, archivar, exportar, mover a papelera.
- Pie: papelera, plugins, ajustes.

### 4.2 Lista de notas

- Cabecera fija: título del libro (editable en línea), fecha/insignia si es diario, contador, botón `+`.
- Selector de orden: **Manual** (por defecto), Título, Creación, Modificación. Dirección ascendente/descendente.
- En orden Manual, cada fila tiene asa de arrastre. En otros órdenes, el asa desaparece y un aviso discreto ofrece «Volver a manual para reordenar».
- Fila: título (1 línea), extracto (2 líneas), fecha relativa, iconos de tipo (imagen, audio, vídeo, dibujo, plugin), indicador de anclaje si la nota tiene bloques libres.
- Estado vacío: «Este libro aún no tiene notas» + «Crear la primera».
- Filtro local por texto; la búsqueda global se hace desde la paleta.
- Selección con clic; `Shift+clic` rango; `Ctrl+clic` múltiple (1.1). En MVP, una nota a la vez para simplificar el arrastre.

### 4.3 Editor

- Cabecera: ruta `Libro / Nota`, título grande editable, estado «Guardado» / «Guardando…» / «Sin guardar», menú `⋯`.
- Barra de formato contextual: estilo de párrafo, negrita, cursiva, subrayado, tachado, código, enlace, color, listas, checklist, cita, tabla, imagen, audio, vídeo, archivo, web, dibujo, plugin.
- Lienzo con fondo papel; la columna de flujo centrada.
- Panel derecho opcional (`Ctrl+Shift+P`): esquema de encabezados, propiedades, historial, plugin.
- Barra inferior: zoom, encuadre, coordenadas del bloque seleccionado, número de palabras.

## 5. Libros diarios

### 5.1 Creación

- Al abrir la app, si `daily.enabled` y no existe el libro del día lógico, se crea según `daily.create`:
  - `first_open` (por defecto): al abrir la app;
  - `first_note`: al crear la primera nota del día;
  - `manual`: solo desde «Crear libro de hoy».
- El libro aparece como «Hoy» y en el árbol por fecha.
- No se crean libros de días no abiertos. Si el usuario quiere rellenar, `Libro → Generar libros faltantes…` con rango y vista previa.
- Si el libro del día fue borrado, no se recrea automáticamente salvo `daily.recreate_deleted = true`.
- **Con la app abierta**, al cruzar `daily.day_start_hour` se recalcula «Hoy» sin interrumpir la nota actual. Si la política es `first_open`, se crea el nuevo libro y se muestra un toast «Nuevo libro del día»; si es `first_note`, se crea al crear la primera nota; si es `manual`, aparece el botón «Crear libro de hoy». La nota abierta sigue siendo la misma hasta que el usuario navegue.
- Un cambio de zona horaria o de reloj recalcula el día lógico y nunca duplica un libro existente; si el nuevo día ya tiene libro, se reutiliza.

### 5.2 Renombrar

- Doble clic en el título o `F2`. Se guarda al perder foco/Enter.
- El título puede ser cualquier texto; la fecha queda como insignia (`29/10/2026`).
- Renombrar no cambia `daily_date`, no crea otro libro y no mueve la carpeta.
- «Restaurar título automático» vuelve a aplicar el patrón.

### 5.3 Navegación temporal

- Botones «Día anterior» / «Día siguiente» en la cabecera del libro diario; si no existe, ofrecen crearlo.
- «Ir a hoy» desde menú o `Ctrl+Shift+H`.
- Calendario mini (1.1) para saltar a una fecha.

## 6. Crear y editar notas

- `Ctrl+N`: nueva nota en el libro actual; si el foco está en «Hoy», en el libro de hoy. Si no hay libro, se crea uno manual.
- La nota nueva abre el título enfocado; el cuerpo vacío muestra «Escribe, o pulsa `/` para insertar…».
- El título se guarda explícito; si se deja vacío, se usa la primera línea del primer bloque de texto (máx. 80 caracteres) solo para mostrar, sin escribir el título.
- Autosave a los 800 ms, al cambiar de nota, al perder foco y al cerrar.
- `Ctrl+S` fuerza guardado y muestra «Guardado».
- Cerrar con cambios pendientes: guarda; nunca pregunta «¿Guardar?» salvo error de disco.
- `Ctrl+Z` / `Ctrl+Shift+Z`: undo/redo de dominio (texto, formato, bloques, canvas, DnD).
- `Ctrl+K`: paleta de comandos (buscar notas, comandos, plugins, ajustes).
- `Ctrl+P`: búsqueda rápida de notas.
- `Ctrl+Shift+E`: previsualización.
- `Ctrl+Shift+F`: modo concentración (oculta sidebar/lista, centra).
- `F11`: pantalla completa.

## 7. Editor de texto

- Columna de 720 px por defecto, configurable 560–960.
- Tipografía del sistema o elegida; tamaño 16 px; interlineado 1.5; márgenes generosos.
- Formato con atajos Markdown: `**negrita**`, `*cursiva*`, `` `código` ``, `# ` heading, `- ` lista, `[] ` checklist, `> ` cita. Se convierten al escribir espacio.
- Menú `/` con búsqueda: Texto, Título 1–3, Lista, Checklist, Cita, Código, Tabla, Imagen, Audio, Vídeo, Archivo, Web, Dibujo, Plugin, Separador.
- Barra flotante al seleccionar texto: negrita, cursiva, subrayado, enlace, color, limpiar formato.
- Pegado:
  - texto plano → texto;
  - HTML → se sanea y convierte a spans/bloques;
  - imagen → asset + bloque;
  - archivo → asset + bloque de archivo;
  - URL → tarjeta web si la preferencia lo permite, si no enlace.
- Arrastrar archivos desde el gestor → asset.
- El texto nunca se renderiza como HTML; Pango evita inyección.
- Ortografía opcional (Enchant/Spellbook), subrayado y sugerencias; desactivada por defecto si no hay diccionario.

## 8. Lienzo híbrido: interacción detallada

### 8.1 Estados de un bloque

| Estado | Apariencia | Interacción |
|---|---|---|
| Flujo | En la columna, sin borde | Clic edita; asa al hover |
| Seleccionado | Borde de acento + asa | Arrastrar, teclado, menú |
| Anclado | Borde punteado sutil + icono de chincheta | Arrastrar libre; «Volver al flujo» |
| Bloque de plugin | Marco y etiqueta del plugin | Editar en panel o inline |
| En error | Borde ámbar + icono | Reintentar/desactivar |

### 8.2 Arrastrar y anclar

1. Hover sobre un bloque muestra un asa de seis puntos a la izquierda.
2. Al arrastrar, el bloque se eleva (sombra), el flujo cierra el hueco con una animación de 150 ms.
3. Al soltar, el bloque queda anclado en `(x, y)` del mundo; aparece la chincheta.
4. Si se suelta dentro de la columna y cerca de un hueco, se ofrece «Volver al flujo» (snap).
5. `Esc` durante el arrastre cancela.
6. Todo el gesto es **un solo comando de undo**.

### 8.3 Recomponer

- Botón en la barra del lienzo y comando `Recomponer nota`.
- Ordena todos los bloques por `(y, x)` visual, los pasa a flujo y limpia anclajes.
- Toast: «Nota recompuesta · Deshacer».
- No pide confirmación porque es reversible; si hay bloques de plugin con tamaño propio, conservan su altura intrínseca.

### 8.4 Volver al flujo (un bloque)

- Acción en el menú contextual o `Ctrl+Shift+R`.
- Inserta el bloque en el hueco de flujo más cercano a su posición vertical actual.
- Si el bloque era el único anclado, la nota vuelve a `mode = flow`.

### 8.5 Navegación y zoom

- `Ctrl+rueda` zoom al puntero; `Ctrl+0` 100 %; `Ctrl+1` encuadre documento; `Ctrl+2` encuadre todo.
- `Espacio+arrastrar` o botón central: paneo.
- Trackpad: dos dedos paneo, pinch zoom.
- Minimapa opcional (1.1); en MVP, barra de desplazamiento y brújula de posición.
- A 25–50 % los bloques muestran preview; clic acerca a 100 % y coloca el cursor si es texto.

### 8.6 Selección múltiple y alineación

- `Shift+clic` añade; lazo en vacío selecciona; `Ctrl+A` selecciona todo el canvas si el foco está en él.
- Mover grupo, alinear izquierda/centro/derecha/arriba/medio/abajo, distribuir horizontal/vertical (1.1).
- Orden Z: traer al frente, enviar al fondo, subir/bajar un nivel.
- Bloquear posición (candado) para evitar arrastres accidentales.
- Duplicar `Ctrl+D`; eliminar `Supr`; copiar/pegar bloques entre notas.

### 8.7 Teclado en el canvas

| Tecla | Acción |
|---|---|
| `Tab` / `Shift+Tab` | Siguiente/anterior bloque |
| `Enter` | Editar bloque seleccionado |
| `Esc` | Salir de edición / deseleccionar |
| Flechas | Mover 1 px si está anclado |
| `Shift+flechas` | Mover 10 px |
| `Alt+flechas` | Redimensionar 1 px |
| `Alt+Shift+flechas` | Redimensionar 10 px |
| `Ctrl+Shift+R` | Volver al flujo |
| `Ctrl+Shift+F/B` | Frente/fondo |
| `Ctrl+0/1/2` | Zoom/encuadres |
| `Supr` | Borrar bloque |

Toda acción tiene equivalente en menú contextual y paleta para accesibilidad.

## 9. Reordenar notas: flujo completo

### 9.1 Dentro del libro

1. Orden Manual activo.
2. Arrastrar el asa de una fila.
3. Línea de inserción azul entre filas; auto-scroll cerca de los bordes.
4. Soltar → se calcula `order_key` entre vecinos, se renombra el archivo, se actualiza el índice.
5. Toast «Nota movida · Deshacer».

### 9.2 Entre libros (spring-load)

1. Arrastrar la nota hacia la barra lateral.
2. El libro bajo el puntero se resalta.
3. Tras `drag.spring_delay_ms` (650 ms), el libro se abre y la lista muestra sus notas.
4. El puntero entra en la lista; aparecen **líneas de inserción** entre notas y un **selector de posición** (popover) con la lista y «Al principio / Al final».
5. Hover sobre una línea resalta la posición; clic o Enter suelta.
6. Si el libro destino está vacío, se muestra una zona «Soltar aquí».
7. `Esc` cancela y devuelve la nota a su sitio.
8. La operación es undoable y atómica.

### 9.3 Alternativa por teclado

- `Ctrl+Shift+M` abre «Mover nota a…».
- Diálogo con búsqueda de libro, lista de posiciones y botones «Al principio / Al final / Después de…».
- `Enter` confirma; `Esc` cancela.
- También `Alt+Arriba/Abajo` mueve dentro del libro en orden manual.

### 9.4 Reglas

- Solo se arrastra en orden Manual. En otros órdenes, el asa se oculta y se ofrece volver.
- Mover no cambia `created_at`; actualiza `updated_at` y `moved_at` (metadato).
- Si el libro destino está archivado, se avisa.
- Si el destino es el mismo libro, se reordena.
- Nunca se duplica la nota: mover es mover.
- Si el `rename` falla (permisos, disco lleno), se revierte el estado visual y se muestra el error.

## 10. Órdenes de notas

| Orden | Criterio | Desempate | Persistencia |
|---|---|---|---|
| Manual | `order_key` del archivo | — | Sí, en el nombre |
| Título | `title` colacionado con ICU | creación | No |
| Creación | `created_at` | ID | No |
| Modificación | `updated_at` | ID | No |
| Tamaño (1.1) | `word_count` | título | No |

- La dirección se recuerda por libro.
- Cambiar de orden no modifica archivos ni el orden manual.
- La búsqueda dentro del libro respeta el orden activo.

## 11. Búsqueda

- `Ctrl+K` abre la paleta con dos modos: comandos y notas.
- La búsqueda global consulta FTS5: título, cuerpo, nombres de adjuntos y etiquetas.
- Resultados con libro, fecha y fragmento resaltado; `Enter` abre; `Ctrl+Enter` abre en panel lateral (1.1).
- Filtros: libro, fecha (hoy/7 días/mes/rango), tipo de bloque, plugin.
- Búsqueda vacía muestra acciones recientes y libros recientes.
- La búsqueda es local; nunca sale de la máquina.
- En bóvedas cifradas solo funciona tras desbloquear.

## 12. Previsualización y concentración

- **Previsualización** (`Ctrl+Shift+E`): oculta asas y barras, renderiza solo lectura, los medios se reproducen, los plugins se muestran en modo lectura. Un botón «Volver a editar» y `Esc`.
- **Concentración** (`Ctrl+Shift+F`): colapsa sidebar y lista, centra la columna, atenúa el resto. `Esc` sale.
- **Pantalla completa** `F11`.
- En previsualización, el canvas conserva posiciones y permite paneo/zoom, pero no edición.
- La previsualización no ejecuta web viva automáticamente; en 1.0 el usuario puede pulsar «Cargar».

## 13. Medios y web en la UI

- Imagen: se muestra con su aspecto; clic abre visor con zoom y fondo; `Esc` cierra.
- Audio: reproductor compacto con onda, play/pausa, tiempo, velocidad (1.0×/1.5×/2×), volumen.
- Vídeo: póster con play; controles al hover; pausa al salir de pantalla.
- Archivo: tarjeta con icono por MIME, nombre, tamaño; «Abrir» usa el portal y pide confirmación la primera vez.
- Web tarjeta: favicon, título, dominio, descripción, botón «Abrir». En 1.0 añade «Cargar vista viva» si está permitido.
- Web viva (1.0): marco con barra de seguridad «Contenido externo · sin cookies», botón de recarga y «Descargar». En el MVP no se muestra la opción.
- Nunca se autoejecuta audio/vídeo con sonido al abrir una nota; solo el usuario inicia.

## 14. Plugins en la UI

- **Gestor**: lista de instalados, versión, permisos, estado, autor, botones activar/desactivar/desinstalar, «Instalar desde archivo…».
- **Detalle**: descripción, permisos con explicación, plugins que lo usan, botón «Abrir carpeta de datos».
- **Insertar**: desde el menú `/` → Plugins → bloque; o desde el panel del plugin.
- **Panel lateral**: pestañas de plugins; se puede fijar.
- **Comandos**: aparecen en `Ctrl+K` con prefijo del plugin.
- **Ajustes**: formulario declarativo; los cambios se aplican en vivo.
- **Errores**: banner en el bloque y en el gestor, con «Reintentar», «Desactivar», «Ver log».
- **Desarrollo**: `folio plugin dev ./mi-plugin` muestra recarga en vivo y log; solo en modo dev.

## 15. Ajustes

- Ventana o diálogo con buscador de ajustes.
- Secciones: General, Apariencia, Editor, Libros diarios, Notas y orden, Lienzo, Medios, Web, Plugins, Bóveda y seguridad, Copias, Atajos, Avanzado.
- Cada control muestra el nombre de la clave TOML y un botón «Copiar ruta».
- «Avanzado» incluye editor TOML con validación, botón «Abrir en editor externo» y «Restaurar valores».
- Cambios con recarga en vivo; los que requieren reinicio se marcan.
- Botón «Exportar configuración» / «Importar configuración» (sin secretos).
- Ajustes por bóveda vs globales claramente indicados.

## 16. Atajos globales

| Atajo | Acción |
|---|---|
| `Ctrl+N` | Nueva nota |
| `Ctrl+Shift+N` | Nuevo libro |
| `Ctrl+K` | Paleta de comandos |
| `Ctrl+P` | Buscar notas |
| `Ctrl+Shift+H` | Ir al libro de hoy |
| `Ctrl+S` | Guardar |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Deshacer/rehacer |
| `Ctrl+Shift+E` | Previsualización |
| `Ctrl+Shift+F` | Concentración |
| `Ctrl+Shift+M` | Mover nota a… |
| `Ctrl+1/2/0` | Encuadre documento/todo/zoom 100 % |
| `F2` | Renombrar |
| `F9` | Alternar barra lateral |
| `Ctrl+,` | Ajustes |
| `Ctrl+Q` | Salir |
| `Ctrl+Shift+?` | Ayuda de atajos |

Los atajos son configurables en `[keybindings]`; los conflictos se detectan y se avisan.

## 17. Accesibilidad

- **Teclado**: todas las acciones tienen atajo o menú; el foco es visible; no hay trampas de foco.
- **Lector de pantalla**: GTK expone roles y nombres; el canvas añade una vista de lista alternativa con acciones.
- **Contraste**: se respetan los tokens de Libadwaita; modo alto contraste del sistema.
- **Escalado**: la UI sigue el escalado de texto de GNOME; el editor permite tamaño propio.
- **Movimiento reducido**: se desactivan animaciones si el sistema lo pide.
- **Colores**: nunca es el único indicador; los estados usan iconos/texto.
- **Canvas**: mover un bloque se puede hacer con flechas o con el diálogo «Mover a coordenadas»; el orden Z y el anclaje tienen comandos.
- **Medios**: subtítulos/transcripciones no entran en MVP; se documenta.
- **Pruebas**: checklist manual por etapa y auditoría con Accerciser en E12.

## 18. Sistema visual

### 18.1 Tokens

| Token | Claro | Oscuro |
|---|---|---|
| Fondo ventana | `#f6f5f4` | `#1e1e1e` |
| Fondo papel | `#ffffff` | `#242424` |
| Texto principal | `#1f1f1f` | `#f2f2f2` |
| Texto secundario | `#6b6b6b` | `#b0b0b0` |
| Acento | `#3584e4` | `#78aeed` |
| Borde | `#deddda` | `#3a3a3a` |
| Sombra bloque | `0 2px 8px rgba(0,0,0,.08)` | `0 2px 10px rgba(0,0,0,.4)` |
| Radio | 10 px | 10 px |
| Espaciado base | 4 px | 4 px |

- Tipografía UI: fuente del sistema (Cantarell/Inter); editor: serif opcional o sans.
- Títulos de nota 28–32 px; cuerpo 16 px; metadatos 12–13 px.
- Iconos simbólicos de Libadwaita; nada de iconos de color salvo estados.
- Animaciones de 120–180 ms, `ease-out`; respetan movimiento reducido.
- Modo claro, oscuro y «seguir al sistema».
- Acento configurable entre los de Libadwaita.

### 18.2 Densidad

- Cómoda por defecto (filas 64 px), compacta opcional (48 px).
- El editor mantiene márgenes amplios; la densidad no afecta la columna de texto.

## 19. Estados vacíos y errores

| Situación | Mensaje | Acción |
|---|---|---|
| Bóveda sin libros | «Tu primera página empieza aquí» | Crear libro / Crear libro de hoy |
| Libro sin notas | «Este libro aún no tiene notas» | Crear la primera |
| Búsqueda sin resultados | «No encontramos “…”» | Limpiar filtros |
| Plugin ausente | «Este bloque usa un plugin que no está instalado» | Buscar/instalar |
| Plugin fallido | «El plugin se detuvo» | Reintentar/desactivar |
| Asset perdido | «No encontramos este archivo» | Reemplazar/eliminar bloque |
| Disco lleno | «No se pudo guardar» | Liberar espacio/exportar |
| Bóveda bloqueada | «Bóveda bloqueada» | Desbloquear |
| Config inválida | «El archivo tiene un error en la línea N» | Abrir TOML / restaurar |

## 20. Criterios de aceptación de UX

- [ ] Crear una nota y escribir requiere ≤ 2 acciones desde el libro.
- [ ] El libro de hoy está visible al abrir; si no existe, se crea en ≤ 1 clic.
- [ ] Renombrar un libro diario no crea duplicados y conserva la fecha.
- [ ] Arrastrar una nota entre libros se completa sin diálogo adicional salvo elegir posición.
- [ ] Spring-load se dispara a los 650 ms y se cancela con `Esc`.
- [ ] Toda acción de canvas tiene equivalente por teclado.
- [ ] La previsualización no permite edición accidental.
- [ ] Ningún texto de la UI se corta a 200 % de escala.
- [ ] El foco es visible en todos los controles.
- [ ] Los errores siempre ofrecen una acción.
