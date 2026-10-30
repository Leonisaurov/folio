# Plugins de referencia

`progress`, `chart`, `diagram`, `stats` y `quickbar` son los plugins de referencia.
`hello` es un plugin de terceros mínimo que se ejecuta con el runtime WASM real.
Cada carpeta contiene su `manifest.toml` y un `plugin.wasm` mínimo de marcador.

El host reconoce estos IDs y ejecuta su implementación de referencia en Rust
usando exactamente el mismo contrato (`Manifest` + `Request`/`Response` +
`UiNode`). Los plugins de terceros se ejecutan con `WasmRuntime` (wasmi, ABI JSON
`api = 1`); ver ADR-033 en `docs/10-decisiones.md` y `plugins/hello`.

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
