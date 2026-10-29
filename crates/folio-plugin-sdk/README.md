# folio-plugin-sdk

Tipos del contrato de plugins: `Manifest`, `Permission`, `Request`, `Response` y
el árbol de UI declarativa `UiNode`, además del WIT en `wit/folio-plugin.wit`.

- Sin HTML ni scripts en la UI: solo nodos tipados.
- Límites: profundidad ≤ 32, nodos ≤ 2000, textos ≤ 10 000 caracteres.
- Permisos explícitos; sin red por defecto.
- El SDK no depende de GTK ni de la aplicación.

Licencia: MIT OR Apache-2.0 (a diferencia del binario, GPL-3.0-or-later).
