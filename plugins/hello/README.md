# Hello WASM

Plugin de terceros mínimo para validar el ABI JSON `api = 1` y el sandbox:

```bash
cargo run -p folio-tools -- compile-wat \
  --input plugins/hello/plugin.wat \
  --output plugins/hello/plugin.wasm
folio --vault /tmp/folio-test plugin install plugins/hello --allow-unsigned
folio --vault /tmp/folio-test plugin test plugins/hello
```

El módulo no importa WASI, no accede al sistema de archivos ni a red y devuelve
un árbol `UiNode` validado por el host. El binario `.wasm` incluido se regenera
desde `plugin.wat` con `folio-tools`.
