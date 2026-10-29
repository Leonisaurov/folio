# Empaquetado Flatpak

El manifiesto usa el runtime GNOME 49 y compila con el SDK de Rust. Para una
construcción reproducible hay que vendorizar las dependencias:

```bash
python3 flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json
flatpak-builder --force-clean --user --install build-dir flatpak/io.github.leonisauro.Folio.yml
flatpak run io.github.leonisauro.Folio --version
```

La vista web viva es una feature opcional (`--features webkit`). Para incluirla
en Flatpak hay que añadir WebKitGTK 6.0 como módulo o usar un runtime que lo
provea, y compilar con `--features webkit`.

`flatpak-cargo-generator.py` viene del repositorio `flatpak/flatpak-builder-tools`.
El manifiesto compila `--offline --locked`: sin `cargo-sources.json` fallará a
propósito en vez de descargar red durante el build.

Pruebas manuales de la puerta E13:

```bash
appstreamcli validate flatpak/io.github.leonisauro.Folio.metainfo.xml
flatpak run io.github.leonisauro.Folio --help
```
