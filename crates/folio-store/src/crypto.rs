//! Cifrado opcional de bóveda: Argon2id + XChaCha20-Poly1305.
//!
//! El formato de archivo cifrado es `FOLIOENC1 || nonce(24) || ciphertext`.
//! La clave de bóveda (VK) nunca se guarda en claro: se envuelve con una clave
//! derivada de la contraseña y se almacena en `.folio/keyring.json`.
//! La frase de recuperación codifica la VK (BIP39, 24 palabras).

use std::fs;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use bip39::Mnemonic;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use folio_core::FolioError;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const MAGIC: &[u8; 9] = b"FOLIOENC1";
const NONCE_LEN: usize = 24;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KdfParams {
    #[serde(default = "default_m")]
    pub m: u32,
    #[serde(default = "default_t")]
    pub t: u32,
    #[serde(default = "default_p")]
    pub p: u32,
}

fn default_m() -> u32 {
    65_536
}
fn default_t() -> u32 {
    3
}
fn default_p() -> u32 {
    4
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m: default_m(),
            t: default_t(),
            p: default_p(),
        }
    }
}

impl KdfParams {
    /// Parámetros rápidos solo para tests.
    pub fn fast() -> Self {
        Self {
            m: 8_192,
            t: 1,
            p: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Keyring {
    pub version: u32,
    pub kdf: String,
    pub salt: String,
    pub m: u32,
    pub t: u32,
    pub p: u32,
    pub nonce: String,
    pub wrapped_key: String,
    /// BLAKE3 de la VK, para verificar la frase de recuperación.
    #[serde(default)]
    pub recovery_hash: String,
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn unb64(text: &str) -> Result<Vec<u8>, FolioError> {
    base64::engine::general_purpose::STANDARD
        .decode(text)
        .map_err(|error| FolioError::Invalid(format!("base64: {error}")))
}

/// Clave de bóveda en memoria; se borra al soltarla.
#[derive(Clone)]
pub struct Cipher {
    key: Zeroizing<[u8; KEY_LEN]>,
}

impl std::fmt::Debug for Cipher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Cipher([redacted])")
    }
}

impl Cipher {
    pub fn from_key(key: [u8; KEY_LEN]) -> Self {
        Self {
            key: Zeroizing::new(key),
        }
    }

    pub fn key_hash(&self) -> String {
        blake3::hash(self.key.as_slice()).to_hex().to_string()
    }

    /// Crea una VK nueva, la envuelve y devuelve también la frase de recuperación.
    pub fn create(
        password: &str,
        params: &KdfParams,
    ) -> Result<(Self, Keyring, String), FolioError> {
        let mut key = [0u8; KEY_LEN];
        getrandom::fill(&mut key)
            .map_err(|error| FolioError::Invalid(format!("entropía: {error}")))?;
        let cipher = Self::from_key(key);
        let keyring = wrap_key(&key, password, params)?;
        let mnemonic = Mnemonic::from_entropy(&key)
            .map_err(|error| FolioError::Invalid(format!("mnemónica: {error}")))?;
        Ok((cipher, keyring, mnemonic.to_string()))
    }

    pub fn unlock(password: &str, keyring: &Keyring) -> Result<Self, FolioError> {
        if keyring.kdf != "argon2id" || keyring.version != 1 {
            return Err(FolioError::Unsupported(format!(
                "keyring versión/kdf no soportada: {} {}",
                keyring.version, keyring.kdf
            )));
        }
        let salt = unb64(&keyring.salt)?;
        let nonce = unb64(&keyring.nonce)?;
        let wrapped = unb64(&keyring.wrapped_key)?;
        if salt.len() != SALT_LEN || nonce.len() != NONCE_LEN || wrapped.len() != KEY_LEN + 16 {
            return Err(FolioError::Corrupt(
                "keyring con longitudes inválidas".into(),
            ));
        }
        let params = Params::new(keyring.m, keyring.t, keyring.p, Some(KEY_LEN))
            .map_err(|error| FolioError::Invalid(format!("Argon2: {error}")))?;
        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut kek = Zeroizing::new([0u8; KEY_LEN]);
        argon
            .hash_password_into(password.as_bytes(), &salt, kek.as_mut())
            .map_err(|error| FolioError::Invalid(format!("Argon2id: {error}")))?;
        let cipher = XChaCha20Poly1305::new((&*kek).into());
        let opened = cipher
            .decrypt(XNonce::from_slice(&nonce), wrapped.as_slice())
            .map_err(|_| FolioError::Invalid("contraseña incorrecta o keyring corrupto".into()))?;
        let key: [u8; KEY_LEN] = opened
            .try_into()
            .map_err(|_| FolioError::Corrupt("VK con longitud inválida".into()))?;
        Ok(Self::from_key(key))
    }

    /// Recupera la VK desde las 24 palabras y comprueba que pertenece al keyring.
    pub fn from_recovery(phrase: &str, keyring: &Keyring) -> Result<Self, FolioError> {
        let mnemonic = Mnemonic::parse(phrase.trim())
            .map_err(|error| FolioError::Invalid(format!("frase de recuperación: {error}")))?;
        let entropy = mnemonic.to_entropy();
        let key: [u8; KEY_LEN] = entropy
            .try_into()
            .map_err(|_| FolioError::Invalid("la frase no contiene 32 bytes".into()))?;
        let cipher = Self::from_key(key);
        let expected = unb64(&keyring.recovery_hash)?;
        let actual = blake3::hash(cipher.key.as_slice());
        if !expected.is_empty() && expected != actual.as_bytes() {
            return Err(FolioError::Invalid(
                "la frase de recuperación no corresponde a esta bóveda".into(),
            ));
        }
        Ok(cipher)
    }

    pub fn encryption_key(&self) -> &[u8; KEY_LEN] {
        &self.key
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, FolioError> {
        let cipher = XChaCha20Poly1305::new((&*self.key).into());
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce)
            .map_err(|error| FolioError::Invalid(format!("nonce: {error}")))?;
        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce), plaintext)
            .map_err(|_| FolioError::Invalid("no se pudo cifrar".into()))?;
        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ciphertext.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    pub fn decrypt(&self, bytes: &[u8]) -> Result<Vec<u8>, FolioError> {
        if !is_encrypted(bytes) {
            return Err(FolioError::Corrupt("falta la cabecera de cifrado".into()));
        }
        let nonce = &bytes[MAGIC.len()..MAGIC.len() + NONCE_LEN];
        let ciphertext = &bytes[MAGIC.len() + NONCE_LEN..];
        let cipher = XChaCha20Poly1305::new((&*self.key).into());
        cipher
            .decrypt(XNonce::from_slice(nonce), ciphertext)
            .map_err(|_| {
                FolioError::Corrupt("descifrado falló: archivo o clave incorrectos".into())
            })
    }
}

pub fn is_encrypted(bytes: &[u8]) -> bool {
    bytes.len() >= MAGIC.len() + NONCE_LEN && &bytes[..MAGIC.len()] == MAGIC
}

fn wrap_key(
    key: &[u8; KEY_LEN],
    password: &str,
    params: &KdfParams,
) -> Result<Keyring, FolioError> {
    let mut salt = [0u8; SALT_LEN];
    getrandom::fill(&mut salt).map_err(|error| FolioError::Invalid(format!("sal: {error}")))?;
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|error| FolioError::Invalid(format!("nonce: {error}")))?;
    let params_argon = Params::new(params.m, params.t, params.p, Some(KEY_LEN))
        .map_err(|error| FolioError::Invalid(format!("Argon2: {error}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params_argon);
    let mut kek = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(password.as_bytes(), &salt, kek.as_mut())
        .map_err(|error| FolioError::Invalid(format!("Argon2id: {error}")))?;
    let cipher = XChaCha20Poly1305::new((&*kek).into());
    let wrapped = cipher
        .encrypt(XNonce::from_slice(&nonce), key.as_slice())
        .map_err(|_| FolioError::Invalid("no se pudo envolver la clave".into()))?;
    Ok(Keyring {
        version: 1,
        kdf: "argon2id".into(),
        salt: b64(&salt),
        m: params.m,
        t: params.t,
        p: params.p,
        nonce: b64(&nonce),
        wrapped_key: b64(&wrapped),
        recovery_hash: b64(blake3::hash(key.as_slice()).as_bytes()),
    })
}

pub fn keyring_path(vault_root: &Path) -> PathBuf {
    vault_root.join(".folio").join("keyring.json")
}

pub fn load_keyring(vault_root: &Path) -> Result<Keyring, FolioError> {
    let path = keyring_path(vault_root);
    let bytes = fs::read(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            FolioError::NotFound("keyring de la bóveda".into())
        } else {
            error.into()
        }
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| FolioError::Corrupt(format!("keyring inválido: {error}")))
}

pub fn save_keyring(vault_root: &Path, keyring: &Keyring) -> Result<(), FolioError> {
    let path = keyring_path(vault_root);
    crate::atomic::write_atomic(&path, &serde_json::to_vec_pretty(keyring)?)?;
    #[cfg(unix)]
    crate::atomic::set_mode(&path, 0o600)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_unlock_and_recovery() {
        let (cipher, keyring, phrase) = Cipher::create("correcta", &KdfParams::fast()).unwrap();
        assert_eq!(phrase.split_whitespace().count(), 24);
        let unlocked = Cipher::unlock("correcta", &keyring).unwrap();
        assert_eq!(cipher.encryption_key(), unlocked.encryption_key());
        assert!(Cipher::unlock("incorrecta", &keyring).is_err());
        let recovered = Cipher::from_recovery(&phrase, &keyring).unwrap();
        assert_eq!(cipher.encryption_key(), recovered.encryption_key());
        let other = Cipher::create("otra", &KdfParams::fast()).unwrap().1;
        assert!(Cipher::from_recovery(&phrase, &other).is_err());
    }

    #[test]
    fn encrypt_decrypt_and_tamper() {
        let (cipher, _, _) = Cipher::create("x", &KdfParams::fast()).unwrap();
        let plaintext = b"titulo secreto y contenido";
        let encrypted = cipher.encrypt(plaintext).unwrap();
        assert!(is_encrypted(&encrypted));
        assert!(!encrypted.windows(plaintext.len()).any(|w| w == plaintext));
        assert_eq!(cipher.decrypt(&encrypted).unwrap(), plaintext);
        let mut broken = encrypted.clone();
        let last = broken.len() - 1;
        broken[last] ^= 1;
        assert!(cipher.decrypt(&broken).is_err());
        assert!(!is_encrypted(b"texto plano"));
    }

    #[test]
    fn keyring_roundtrip_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let (_, keyring, _) = Cipher::create("x", &KdfParams::fast()).unwrap();
        save_keyring(dir.path(), &keyring).unwrap();
        let loaded = load_keyring(dir.path()).unwrap();
        assert_eq!(loaded, keyring);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(keyring_path(dir.path()))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
    }
}
