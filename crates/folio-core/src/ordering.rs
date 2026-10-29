//! Claves de orden fraccional base62 de longitud fija.
//!
//! El alfabeto está en orden ASCII y las claves se codifican como números de 12
//! dígitos. La longitud fija evita que el nombre de archivo crezca sin límite al
//! insertar notas (un fallo detectado por el soak test: `ENAMETOOLONG`).

use crate::error::FolioError;

pub const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE: u128 = 62;
pub const KEY_LEN: usize = 12;
const MAX_VALUE: u128 = 3_226_266_762_397_899_821_055; // 62^12 - 1
/// Separación entre claves consecutivas: deja ~20 puntos medios antes de
/// necesitar rebalanceo y permite 10k notas sin acercarse al máximo.
const STEP: u128 = 1_000_000;
/// Clave inicial en el centro del espacio: deja sitio a ambos lados.
pub const FIRST_KEY: &str = "V00000000000";

fn digit_index(byte: u8) -> Option<u128> {
    ALPHABET
        .iter()
        .position(|candidate| *candidate == byte)
        .map(|index| index as u128)
}

fn digit(index: u128) -> char {
    ALPHABET[(index % BASE) as usize] as char
}

/// Codifica un valor en exactamente 12 caracteres base62.
pub fn encode(mut value: u128) -> Option<String> {
    if value > MAX_VALUE {
        return None;
    }
    let mut bytes = [b'0'; KEY_LEN];
    for slot in bytes.iter_mut().rev() {
        *slot = digit(value % BASE) as u8;
        value /= BASE;
    }
    Some(String::from_utf8(bytes.to_vec()).expect("alfabeto ASCII"))
}

/// Decodifica una clave de hasta 12 caracteres.
pub fn decode(key: &str) -> Option<u128> {
    if key.is_empty() || key.len() > KEY_LEN || !key.is_ascii() {
        return None;
    }
    let mut value: u128 = 0;
    for byte in key.bytes() {
        value = value.checked_mul(BASE)?.checked_add(digit_index(byte)?)?;
    }
    Some(value)
}

pub fn is_valid_key(key: &str) -> bool {
    decode(key).is_some()
}

/// Devuelve una clave estrictamente entre `prev` y `next`.
///
/// `None` significa «sin límite». Devuelve `None` si no queda espacio y el
/// libro necesita rebalanceo.
pub fn key_between(prev: Option<&str>, next: Option<&str>) -> Option<String> {
    let low = match prev {
        Some(key) => decode(key)?,
        None => 0,
    };
    let high = match next {
        Some(key) => decode(key)?,
        None => MAX_VALUE,
    };
    if high <= low {
        return None;
    }
    if prev.is_none() && next.is_none() {
        return Some(FIRST_KEY.to_string());
    }
    let candidate = if prev.is_none() {
        // No devolver la clave cero si hay sitio para algo mayor.
        let value = high / 2;
        if value == 0 { 1 } else { value }
    } else {
        low + (high - low) / 2
    };
    if candidate <= low || candidate >= high {
        return None;
    }
    encode(candidate)
}

/// Genera una clave después de `prev`.
pub fn key_after(prev: Option<&str>) -> Option<String> {
    let value = match prev {
        Some(key) => decode(key)?.checked_add(STEP)?,
        None => decode(FIRST_KEY)?.checked_add(STEP)?,
    };
    encode(value)
}

/// Genera una clave antes de `next`.
pub fn key_before(next: Option<&str>) -> Option<String> {
    let value = match next {
        Some(key) => decode(key)?.checked_sub(STEP)?,
        None => decode(FIRST_KEY)?.checked_sub(STEP)?,
    };
    encode(value)
}

/// Valida una secuencia completa; usada por `doctor` y por los tests.
pub fn validate_sequence(keys: &[String]) -> Result<(), FolioError> {
    for window in keys.windows(2) {
        if window[0] >= window[1] {
            return Err(FolioError::Conflict(format!(
                "claves de orden desordenadas o duplicadas: {} >= {}",
                window[0], window[1]
            )));
        }
    }
    for key in keys {
        if !is_valid_key(key) {
            return Err(FolioError::Invalid(format!(
                "clave de orden inválida: {key}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_keys_are_ordered_and_fixed_length() {
        let a = key_between(None, None).unwrap();
        assert_eq!(a, FIRST_KEY);
        assert_eq!(a.len(), KEY_LEN);
        let b = key_between(Some(&a), None).unwrap();
        let c = key_between(Some(&b), None).unwrap();
        assert!(a < b && b < c);
        assert_eq!(b.len(), KEY_LEN);
        assert_eq!(c.len(), KEY_LEN);
    }

    #[test]
    fn insert_at_start_and_middle() {
        let a = FIRST_KEY.to_string();
        let b = key_between(Some(&a), None).unwrap();
        let first = key_between(None, Some(&a)).unwrap();
        assert!(first < a, "{first} < {a}");
        let middle = key_between(Some(&a), Some(&b)).unwrap();
        assert!(a < middle && middle < b, "{a} < {middle} < {b}");
    }

    #[test]
    fn ten_thousand_insertions_keep_length_bounded() {
        let mut keys = vec![key_between(None, None).unwrap()];
        for _ in 0..10_000 {
            let last = keys.last().unwrap().clone();
            let next = key_after(Some(&last)).unwrap();
            assert!(next.len() <= KEY_LEN);
            keys.push(next);
        }
        for window in keys.windows(2) {
            assert!(window[0] < window[1], "{} < {}", window[0], window[1]);
        }
        validate_sequence(&keys).unwrap();
        assert!(keys.iter().all(|key| key.len() == KEY_LEN));
    }

    #[test]
    fn many_start_insertions_stay_bounded() {
        let mut first = key_between(None, None).unwrap();
        for _ in 0..1000 {
            let Some(new_first) = key_between(None, Some(&first)) else {
                // Agotar el hueco izquierdo es posible; la capa superior
                // rebalancea. Lo importante es no generar claves gigantes.
                break;
            };
            assert!(new_first < first);
            assert_eq!(new_first.len(), KEY_LEN);
            first = new_first;
        }
    }

    #[test]
    fn rejects_inverted_bounds() {
        assert!(key_between(Some("V00000000001"), Some("V00000000000")).is_none());
        assert!(key_between(Some("V00000000000"), Some("V00000000000")).is_none());
    }

    #[test]
    fn roundtrip_encode_decode() {
        for value in [0, 1, 30, 61, 62, 123456, MAX_VALUE] {
            let key = encode(value).unwrap();
            assert_eq!(decode(&key), Some(value));
            assert_eq!(key.len(), KEY_LEN);
        }
        assert!(encode(MAX_VALUE + 1).is_none());
        assert!(decode("demasiado larga").is_none());
    }
}
