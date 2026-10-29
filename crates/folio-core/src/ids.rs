use uuid::Uuid;

/// UUIDv7: ordenable por tiempo y generado en el cliente.
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub fn is_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok()
}

/// Identificador corto y estable para nombres de carpeta.
pub fn short_slug(id: &str) -> String {
    let hex: String = id.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let start = hex.len().saturating_sub(6);
    hex[start..].to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_valid_and_ordered() {
        let a = new_id();
        let b = new_id();
        assert!(is_uuid(&a));
        assert!(is_uuid(&b));
        assert!(a < b, "UUIDv7 debe ordenar por tiempo");
    }

    #[test]
    fn slug_is_short() {
        let id = new_id();
        let slug = short_slug(&id);
        assert_eq!(slug.len(), 6);
        assert!(slug.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
