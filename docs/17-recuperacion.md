# 17 — Recuperación, backups y diagnóstico

## 1. Regla de oro

La bóveda son archivos. La app puede desaparecer y los datos siguen ahí. El índice
SQLite es desechable. Los assets son content-addressed y no se borran al borrar una
nota hasta que la recolección lo permita.

## 2. Si Folio no arranca

```bash
folio --vault /ruta/a/la/boveda doctor --rebuild-index
folio --vault /ruta/a/la/boveda doctor --json
```

`doctor`:

1. recorre libros y notas;
2. valida JSON e invariantes;
3. mueve archivos corruptos a `.folio/quarantine/` (nunca los borra);
4. reconstruye el índice si se pide;
5. reporta assets huérfanos o ausentes y operaciones pendientes.

Para probar sin tocar la bóveda real, copia la carpeta y ejecuta sobre la copia.

## 3. Recuperación tras un crash

- Cada cambio escribe `.folio/recovery/<note-id>.json`.
- El guardado normal borra ese snapshot.
- Al reabrir, si el snapshot es más nuevo que la nota en disco, Folio lo carga y
  muestra «Recuperada una edición no guardada».
- El archivo final nunca se escribe a medias: temporal + `fsync` + `rename` +
  `fsync` del directorio.

## 4. Papelera

- Borrar una nota la mueve a `.folio/trash/<timestamp>-<id>/` con `meta.json`.
- Retención por defecto: 30 días; configurable en `notes.trash_days`.
- Restaurar devuelve a la ruta original; si el libro ya no existe, la app pide
  destino al usuario (CLI usa una ruta segura dentro de la bóveda).
- Vaciar la papelera no toca assets referenciados.

CLI:

```bash
folio --vault ~/Notas trash list
folio --vault ~/Notas trash restore .folio/trash/...
folio --vault ~/Notas trash purge --days 30
```

## 5. Versiones

- Snapshot comprimido zstd en `.folio/versions/<note-id>/<epoch>.json.zst`.
- Se crea al guardar si pasaron ≥ 5 min, o al cerrar la nota.
- Retención: 50 versiones o 30 días (se configura en `notes.version_*`).
- Restaurar crea una versión nueva; no borra la actual.

```bash
folio --vault ~/Notas note versions <note-id>
```

## 6. Backups

- Snapshot por hardlinks en `.folio/backups/<timestamp>/`.
- Incluye `vault.toml`, `books/`, `assets/` y `plugins/`; excluye índice, papelera
  y los propios backups.
- `manifest.json` guarda BLAKE3 de cada archivo.
- Restaurar **siempre sobre una copia**, nunca in-place.

```bash
folio --vault ~/Notas backup create
folio --vault ~/Notas backup list
folio --vault ~/Notas backup verify
folio --vault ~/Notas backup verify .folio/backups/2026-...
folio --vault ~/Notas backup restore .folio/backups/... /tmp/boveda-restaurada
```

En un sistema de archivos sin hardlinks se copian los archivos; el backup sigue
siendo verificable.

## 7. Edición externa

- Nota cerrada editada fuera: el watcher reindexa y la lista se actualiza.
- Nota abierta sin cambios locales: se recarga con aviso.
- Nota abierta con cambios: Folio no sobrescribe en silencio; conserva el buffer y
  ofrece guardar una copia.
- Archivo borrado fuera: la nota pasa a huérfana y se ofrece guardar como nueva.

## 8. Instancia única y solo lectura

`.folio/lock` contiene el PID. Si otra instancia viva lo tiene, la segunda no
escribe. Un lock de un proceso muerto se recupera automáticamente.

## 9. Bóveda cifrada

- Clave: `VK` de 32 bytes; `KEK = Argon2id(contraseña, sal, 64 MiB, t=3, p=4)`.
- `.folio/keyring.json` contiene sal, parámetros, nonce y VK envuelta; permisos 0600.
- Cada archivo de datos: `FOLIOENC1 || nonce(24) || ciphertext`.
- La frase de recuperación BIP39 de 24 palabras codifica la VK. El keyring guarda
  un hash para rechazar frases de otra bóveda.
- El índice se reconstruye en memoria tras desbloquear; no hay `index.sqlite` en
  claro. La caché de assets descifrados se borra al bloquear.
- Si pierdes contraseña y frase, no hay recuperación posible. Guarda la frase fuera
  de la bóveda.
- Los backups de una bóveda cifrada siguen cifrados. Los backups antiguos en claro
  deben cifrarse o eliminarse manualmente.

```bash
folio --vault ~/Notas --recovery "palabra1 palabra2 … palabra24" note list --book today
```

## 10. Caos y verificación

```bash
./scripts/chaos.sh       # kill -9 en 20 guardados y 30 movimientos
./scripts/security.sh
./scripts/perf.sh
./scripts/soak.sh --quick 30
```

Cada prueba crea su propia bóveda temporal. Nunca se usa `~/Notas`.
