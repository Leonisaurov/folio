# 24 — Crear la clave GPG y firmar la release

## 1. Instalar GnuPG

```bash
# Arch
sudo pacman -S gnupg
# Debian/Ubuntu
sudo apt install gnupg
```

## 2. Crear la clave

Recomendado: clave **RSA 4096 solo de firma**, con caducidad (2 años) y
contraseña. La contraseña se pide al firmar y la guarda `gpg-agent`; no se escribe
en el repositorio.

```bash
gpg --full-generate-key
```

Elige:

- tipo `RSA and RSA` (o `ECC (sign only)` con ed25519 si prefieres);
- 4096 bits;
- caducidad `2y`;
- nombre real del proyecto/persona y correo de release;
- contraseña segura.

Obtén la huella:

```bash
gpg --list-secret-keys --keyid-format=long
# copia la huella de 40 hex de la línea "fpr" (o el key id largo)
```

## 3. Crear una clave sin contraseña (solo CI/pruebas)

No la uses para una release pública. En un `GNUPGHOME` aislado:

```bash
export GNUPGHOME="$(mktemp -d)"
gpg --batch --generate-key <<'EOF'
%no-protection
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name-Real: Folio Release Test
Name-Email: releases@example.invalid
Expire-Date: 0
%commit
EOF
gpg --list-secret-keys --with-colons | awk -F: '/^fpr:/{print $10; exit}'
```

## 4. Exportar la clave pública

```bash
FPR="<huella>"
gpg --armor --export "$FPR" > folio-release-key.asc
```

Publica `folio-release-key.asc` en el repositorio y, opcionalmente, súbela a
`keys.openpgp.org`:

```bash
gpg --send-keys "$FPR"
```

Nunca subas la clave privada. Si usas GitHub Actions, guarda la privada como
secreto `GPG_PRIVATE_KEY` y la contraseña como `GPG_PASSPHRASE`; importa la clave
solo dentro del job.

## 5. Firmar los artefactos

Con las claves de release ya generadas:

```bash
FOLIO_SIGNING_KEY="$FPR" ./scripts/sign-artifacts.sh
```

Firma:

- `dist/folio-1.0.0-linux-x86_64.tar.gz`
- `dist/folio_1.0.0_amd64.deb`
- `dist/arch/folio-1.0.0-1-x86_64.pkg.tar.zst`
- `dist/RELEASE-1.0.0.md`
- `dist/SHA256SUMS`

`release.sh` ya firma el tarball si `FOLIO_SIGNING_KEY` está definida.
`sign-artifacts.sh` firma el resto y crea `SHA256SUMS.asc`.

## 6. Verificar

```bash
gpg --import folio-release-key.asc
gpg --verify dist/folio-1.0.0-linux-x86_64.tar.gz.asc \
             dist/folio-1.0.0-linux-x86_64.tar.gz
gpg --verify dist/folio_1.0.0_amd64.deb.asc dist/folio_1.0.0_amd64.deb
gpg --verify dist/SHA256SUMS.asc dist/SHA256SUMS
sha256sum -c dist/SHA256SUMS
```

Si `gpg --verify` dice «Good signature» y la huella coincide con la publicada,
la release está firmada. Una beta puede publicarse sin firma, pero debe indicarlo.

## 7. Hardware key

Para una release pública, usa una YubiKey/Nitrokey con la clave privada dentro.
`FOLIO_SIGNING_KEY` es la misma huella; `gpg` pedirá el PIN del dispositivo. No
exportes la clave privada de un hardware key.
