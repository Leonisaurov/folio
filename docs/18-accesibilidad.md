# 18 — Accesibilidad

## 1. Objetivo

Que Folio se pueda usar con teclado, lector de pantalla, escala 200 % y contraste
alto sin perder funciones del editor ni del lienzo.

## 2. Teclado y foco

- Todas las acciones principales están en menú y tienen atajo.
- `Tab`/`Shift+Tab` recorren sidebar, lista de notas, título, barra de formato,
  editor y controles de bloque.
- El título recibe el foco al abrir una nota; si no hay nota, lo recibe la búsqueda.
- `Esc` en el lienzo vuelve al documento.
- Los botones de icono exponen `Property::Label`; si Libadwaita crea uno interno,
  se le asigna una etiqueta de respaldo.
- `Ctrl+Shift+B` abre la **lista de bloques**, alternativa al lienzo con acciones
  subir, bajar, anclar/al flujo, editar y eliminar.

## 3. Lienzo alternativo

La lista de bloques es la alternativa oficial: no exige ratón, muestra el tipo y
el primer renglón de cada bloque, y permite cambiar su orden o su anclaje. La
previsualización de solo lectura también sirve como repaso sin edición.

## 4. Escala y contraste

- La escala 200 % se prueba con `GDK_DPI_SCALE=2` en `scripts/a11y.sh`.
- El tema claro/oscuro se aplica con `AdwStyleManager` y se prueba en vivo.
- Los colores de texto usan los tokens del tema; los estados vacíos y errores
  llevan texto, no solo color.
- `ui.reduce_motion` está previsto en el esquema; la interfaz evita animaciones
  largas y no bloquea la lectura.

## 5. Lector de pantalla

GTK4 aporta roles y textos accesibles. Folio añade etiquetas a botones de icono,
listas, buscador, título, editor y bloques. La comprobación automatizada ejecuta el
puente de accesibilidad en Xvfb; **la revisión manual con Orca en GNOME/Wayland es
parte del checklist de release** y no se sustituye por el test.

## 6. Verificación

```bash
./scripts/a11y.sh
./scripts/automation/all.sh
```

El test comprueba:

- etiquetas de botones de icono;
- foco lógico y visibilidad del título;
- tema oscuro/claro en vivo;
- apertura de la lista de bloques;
- ejecución con `GDK_DPI_SCALE=2`.

## 7. Pendiente para el cierre de 1.0

- Revisión manual con Orca y navegación completa en Wayland.
- Checklist visual a 200 % en claro/oscuro.
- Prueba con usuario externo y teclado alternativo.
