# Clave de release

- Huella: `18615FBFBDFAE2E51866B5AB554741526478CD4A`
- Identidad: `Leonisaurov <leonisaurov@gmail.com>`
- Uso: firma de artefactos de release.
- Archivo público: `folio-release-key.asc`.

Importar y verificar:

```bash
gpg --import folio-release-key.asc
gpg --verify dist/folio-1.0.0-linux-x86_64.tar.gz.asc \
             dist/folio-1.0.0-linux-x86_64.tar.gz
```

La clave privada **no está** en el repositorio. Se generó sin passphrase para el RC;
añádela con `gpg --edit-key 18615FBFBDFAE2E51866B5AB554741526478CD4A` (`passwd`)
antes de publicar. Publica también la huella en un canal independiente (web,
Keybase, keys.openpgp.org) para que los testers puedan confiar en ella.
