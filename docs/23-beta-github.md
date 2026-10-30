# Beta pública en GitHub

## Invitación

Gracias por probar Folio. La beta es **local primero y sin telemetría**: no se envía
ningún dato a ningún servidor. El feedback se hace por issues de GitHub o por el
canal que indique el mantenedor.

## Descargar

En la página de releases del repositorio, busca el pre-release `v1.0.0-beta.*` y
elige:

- **Debian/Ubuntu**: `folio_1.0.0_amd64.deb`
- **Arch/pacman**: `folio-1.0.0-1-x86_64.pkg.tar.zst` (o compila con `PKGBUILD`)
- **Cualquier Linux x86_64**: `folio-1.0.0-linux-x86_64.tar.gz`

Comprueba `SHA256SUMS` antes de instalar. Los artefactos de la beta pueden no estar
firmados; no los instales si tu política exige firma.

## Qué probar durante 30 días

Sigue `docs/20-beta-30-dias.md`. En resumen:

1. Crea una bóveda de prueba (no uses datos únicos sin backup).
2. Prueba libros diarios, editor, lienzo, DnD, medios, tablas y dibujo.
3. Exporta MD/HTML/PDF e importa Markdown.
4. Prueba backup/restore, papelera, versiones y `doctor`.
5. Si usas cifrado, guarda la frase de recuperación fuera de la bóveda.
6. Prueba plugins de referencia y, si quieres, `plugins/hello`.
7. Reporta P0/P1 con la plantilla beta y adjunta `folio doctor --json` sin datos
   personales.

## Instalación

```bash
sudo apt install ./folio_1.0.0_amd64.deb     # Debian/Ubuntu
sudo pacman -U folio-1.0.0-1-x86_64.pkg.tar.zst  # Arch
# o extraer el tarball y ejecutar ./folio
```

## Feedback y seguridad

- No publiques bóvedas reales. Usa una copia o fixtures.
- Para P0 (pérdida/corrupción/seguridad) usa el asunto `P0` y detén las pruebas.
- No hay telemetría, crash reporting remoto ni actualizaciones automáticas.
- La beta no sustituye a la revisión manual con Orca/Wayland ni a la validación
  Flatpak/AUR en máquina limpia.
