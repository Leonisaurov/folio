# 16 — Manual de usuario

## 1. Abrir Folio

```bash
folio                 # última bóveda o bienvenida
folio --vault ~/Notas # bóveda concreta
```

La primera vez, «Crear bóveda» pide una carpeta. La bóveda es una carpeta normal:
puedes copiarla, inspeccionarla y respaldarla sin Folio.

## 2. Libros

- **Hoy** (`Ctrl+Shift+H`) abre el libro del día lógico. Por defecto el día empieza
  a las 04:00 locales, así que una nota de las 03:59 pertenece al día anterior.
- **Nuevo libro** (`Ctrl+Shift+N`) crea un libro manual.
- Renombrar un libro no cambia su `daily_date` ni crea duplicados.
- El menú permite archivar, mover notas y crear backups.
- `folio backfill --from 2026-10-01 --to 2026-10-31` genera los libros diarios que
  falten sin tocar los existentes.

## 3. Notas

- `Ctrl+N` crea una nota en el libro actual.
- El orden es manual. Arrastra una nota dentro de la lista para reordenarla o a un
  libro de la barra lateral para moverla. Si te quedas 650 ms sobre un libro, se
  abre para elegir el destino.
- «Mover nota a…» (`Ctrl+Shift+M`) elige libro, posición y nota de referencia.
- `Ctrl+Delete` mueve a la papelera; se puede restaurar desde el menú.

## 4. Escribir

| Atajo | Acción |
|---|---|
| `Ctrl+B` / `Ctrl+I` / `Ctrl+U` | Negrita, cursiva, subrayado en la barra |
| `# `, `## `, `### ` | Encabezados (atajos Markdown) |
| `- `, `- [ ] `, `- [x] ` | Lista y checklist |
| `> ` | Cita |
| `` `texto` `` | Código inline |
| `/` | Menú de inserción (si está activo) |
| `Ctrl+Shift+E` | Previsualización de solo lectura |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Deshacer / rehacer |
| `Ctrl+1` / `Ctrl+2` | Encuadre documento / todo |
| `Ctrl+0` | Zoom 100 % |

El autosave espera 800 ms (configurable). El estado «Guardado» solo aparece cuando
el archivo está en disco. Cada cambio deja un snapshot de recuperación.

## 5. Bloques

El botón `+` del editor inserta texto, tabla, dibujo, imagen, archivo, audio, vídeo
o tarjeta web. Cada bloque tiene:

- **↑ / ↓**: mover el bloque.
- **Anclar / Volver al flujo**: pasar de flujo a coordenadas libres o al revés.
- **Duplicar** y **Eliminar**.

En modo **Lienzo** arrastra un bloque para anclarlo; «Recomponer» ordena todo por
posición visual y devuelve al flujo. El zoom va de 25 % a 400 %.

Para tablas: escribe en las celdas o pega CSV/TSV desde el portapapeles. Se guarda
como celdas con texto.

Para dibujar: arrastra el ratón o stylus sobre el bloque. Cada trazo se guarda como
puntos; la exportación HTML conserva vectores SVG.

## 6. Medios y web

- Imagen/archivo/audio/vídeo se copian a `assets/` con deduplicación BLAKE3.
- «Abrir» en un archivo pide confirmación y usa la aplicación del sistema.
- La tarjeta web valida la URL (rechaza `javascript:`, `file:` y `data:`) y
  funciona sin red con los metadatos cacheados o un fallback.
- La vista web viva es 1.0.

## 7. Buscar, previsualizar y exportar

- `Ctrl+P` enfoca la búsqueda global (FTS5, sin distinguir acentos).
- La previsualización no permite editar.
- Exporta Markdown o HTML desde el menú. El HTML es autocontenido, sin scripts y
  conserva las posiciones de los bloques anclados.
- Importa Markdown desde el menú; el HTML crudo se descarta.

## 8. Plugins

Menú → **Plugins**. Puedes insertar un bloque de los plugins de referencia y
habilitar/deshabilitar los instalados. Cada plugin declara permisos; sin `network`
no puede hacer HTTP. Si un plugin falla, el bloque muestra el error y el estado se
conserva.

CLI:

```bash
folio plugin list
folio plugin test plugins/chart
folio --vault ~/Notas plugin install plugins/chart --allow-unsigned
folio --vault ~/Notas plugin disable io.github.folio.chart
```

## 9. Ajustes

Menú → **Ajustes** (`Ctrl+,`). La ventana se genera del esquema: busca por nombre,
ruta o grupo. Los cambios se escriben en `vault.toml` o en el `config.toml` global
según el ámbito, preservando comentarios. Un valor inválido no se aplica.

También puedes editar el TOML o usar:

```bash
folio config path --vault
folio config get daily
folio config set daily.title_format "%Y/%m/%d"
folio config validate
folio config schema
```

Precedencia: CLI > `FOLIO_*` > bóveda > global > defectos.

## 10. Tema y accesibilidad

El tema sigue al sistema o se fuerza claro/oscuro en `ui.theme`. Toda acción
principal tiene atajo y entrada de menú. La previsualización y el lienzo tienen
alternativa por teclado. El informe completo de accesibilidad es 1.0.

## 11. Datos

```text
vault/
├── vault.toml
├── books/<fecha>_<slug>/{book.toml,notes/<order>_<id>.json}
├── assets/ab/cd/<blake3>.<ext>
├── plugins/
└── .folio/{index.sqlite,backups,trash,versions,quarantine,recovery}
```

Puedes borrar `.folio/index.sqlite` en cualquier momento: se reconstruye. Los JSON
mandan.
