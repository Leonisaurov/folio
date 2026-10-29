# Política de seguridad

## Versiones soportadas

| Versión | Soporte |
|---|---|
| 0.1.x (MVP) | Sí, hasta la 1.0 |
| < 0.1 | No |

## Cómo reportar

**No abras un issue público con datos personales ni una prueba de concepto destructiva.**

- Usa GitHub Security Advisories (Private vulnerability reporting) en el repositorio.
- Si no está disponible, escribe a `security@folio.invalid` con asunto `SECURITY`.
- Incluye: versión, distro, pasos reproducibles, impacto y si afecta a datos.

Responderemos en un plazo objetivo de 72 h y coordinaremos la divulgación.

## Modelo de amenazas (resumen)

Folio trata como no confiable:

- el contenido importado (Markdown/HTML);
- los plugins;
- las URL de bloques web;
- los archivos de la bóveda editados fuera de la app;
- la red.

Controles principales:

- escritura atómica + fsync + rename, journal y backups verificables;
- índice SQLite reconstruible; nunca fuente de verdad;
- rutas confinadas a la bóveda y rechazo de `..`/enlaces simbólicos en assets;
- HTML de exportación sin scripts; importación que descarta HTML crudo;
- plugins con permisos explícitos, sin red por defecto, límites de memoria/tiempo
  y UI declarativa validada;
- enlaces externos con confirmación y `xdg-open`/portal;
- cifrado de bóveda planificado para 1.0 (XChaCha20-Poly1305 + Argon2id).

## Pruebas

```bash
cargo test -p folio-plugins
cargo test -p folio-store assets
./scripts/security.sh
```

No se aceptan secretos en `config.toml` ni en `vault.toml`; usa el keyring.
