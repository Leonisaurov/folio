# Plugins de referencia

`progress`, `chart`, `diagram`, `stats` y `quickbar` son los plugins de ejemplo.
Cada carpeta contiene su `manifest.toml` y un `plugin.wasm` mínimo de marcador.

En esta compilación del MVP el host reconoce estos IDs y ejecuta su
implementación de referencia en Rust usando exactamente el mismo contrato
(`Manifest` + `Request`/`Response` + `UiNode`). El cargador WASM de terceros
(Wasmtime + ABI JSON) queda para 1.0; ver ADR-021 en `docs/10-decisiones.md`.

## Probar

```bash
folio plugin list
folio plugin test plugins/chart
folio --vault /tmp/demo plugin install plugins/chart --allow-unsigned
```

## Crear uno

1. Copia `crates/folio-plugin-sdk/wit/folio-plugin.wit` como contrato.
2. Define `manifest.toml` con `id` inverso, versión semver, `api = 1`,
   bloques/comandos y permisos mínimos.
3. Implementa `render`/comandos y devuelve un árbol `UiNode` validado.
4. Firma el paquete con Ed25519 o usa `allow_unsigned` solo en desarrollo.
5. Ejecuta `folio plugin test <carpeta>`.
