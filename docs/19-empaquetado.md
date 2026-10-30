# 19 — Empaquetado `.deb`

## 1. Construcción

```bash
./scripts/build-deb.sh
```

El script:

1. compila `folio` (con `--features webkit` si WebKitGTK 6.0 está disponible) y
   `folio-tools` en release;
2. crea el árbol FHS en un directorio temporal;
3. escribe `DEBIAN/control` con dependencias y `Installed-Size`;
4. empaqueta con `dpkg-deb` si existe; si no, con `scripts/deb_build.py`
   (`ar` + `control.tar.gz` + `data.tar.gz`);
5. verifica extrayendo el paquete, comprobando archivos y permisos, ejecutando
   `folio --version` y validando AppStream.

Artefacto:

```text
dist/folio_<versión>_<arquitectura>.deb
```

## 2. Contenido

```text
/usr/bin/folio
/usr/bin/folio-tools
/usr/share/applications/io.github.leonisauro.Folio.desktop
/usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml
/usr/share/icons/hicolor/scalable/apps/io.github.leonisauro.Folio.svg
/usr/share/doc/folio/manual.md
/usr/share/doc/folio/recuperacion.md
/usr/share/doc/folio/copyright
```

## 3. Dependencias

`libgtk-4-1`, `libadwaita-1-0`, `libsqlite3-0`, `libcairo2`,
`libpango-1.0-0`, `libgstreamer1.0-0` y, si se compiló la feature,
`libwebkitgtk-6.0-4`.

## 4. Verificación manual en VM limpia

```bash
sudo dpkg -i dist/folio_1.0.0_amd64.deb
folio --version
folio vault init ~/Notas
flatpak run io.github.leonisauro.Folio --version  # si se publica Flatpak
appstreamcli validate /usr/share/metainfo/io.github.leonisauro.Folio.metainfo.xml
```

La instalación real en una VM limpia sigue siendo una puerta manual de E13; el
script verifica estructura, ejecutable y metadatos en el entorno de desarrollo.
