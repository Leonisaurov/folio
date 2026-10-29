use jiff::Timestamp;

use crate::error::FolioError;
use crate::model::{
    BlockContent, MAX_BLOCKS, MAX_NOTE_BYTES, MAX_PLUGIN_STATE_BYTES, MAX_TITLE_CHARS, Note,
    SCHEMA_VERSION, TEXT_MARKS,
};

impl Note {
    /// Valida las invariantes del modelo. Devuelve el primer error con contexto.
    pub fn validate(&self) -> Result<(), FolioError> {
        if self.schema > SCHEMA_VERSION {
            return Err(FolioError::Unsupported(format!(
                "esquema {} mayor que {}; actualiza Folio",
                self.schema, SCHEMA_VERSION
            )));
        }
        if self.schema == 0 {
            return Err(FolioError::Invalid("schema debe ser >= 1".into()));
        }
        if self.id.trim().is_empty() || self.id.len() > 64 {
            return Err(FolioError::Invalid(
                "id de nota vacío o demasiado largo".into(),
            ));
        }
        if self.title.chars().count() > MAX_TITLE_CHARS {
            return Err(FolioError::Invalid(format!(
                "título de más de {MAX_TITLE_CHARS} caracteres"
            )));
        }
        let created: Timestamp = self.created_at.parse().map_err(|_| {
            FolioError::Invalid(format!("created_at no es RFC 3339: {}", self.created_at))
        })?;
        let updated: Timestamp = self.updated_at.parse().map_err(|_| {
            FolioError::Invalid(format!("updated_at no es RFC 3339: {}", self.updated_at))
        })?;
        if updated < created {
            return Err(FolioError::Invalid(
                "updated_at es anterior a created_at".into(),
            ));
        }
        if self.blocks.len() > MAX_BLOCKS {
            return Err(FolioError::Invalid(format!(
                "{} bloques supera el máximo de {MAX_BLOCKS}",
                self.blocks.len()
            )));
        }
        if !self.layout.flow_width.is_finite() || self.layout.flow_width <= 0.0 {
            return Err(FolioError::Invalid("flow_width inválido".into()));
        }

        let mut seen = std::collections::HashSet::new();
        for block in &self.blocks {
            if block.id.trim().is_empty() {
                return Err(FolioError::Invalid("bloque sin id".into()));
            }
            if !seen.insert(block.id.as_str()) {
                return Err(FolioError::Invalid(format!(
                    "id de bloque duplicado: {}",
                    block.id
                )));
            }
            if let Some((x, y, w, h, _z)) = block.placement.rect() {
                if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
                    return Err(FolioError::Invalid(format!(
                        "coordenadas no finitas en bloque {}",
                        block.id
                    )));
                }
                if w <= 0.0 || h <= 0.0 {
                    return Err(FolioError::Invalid(format!(
                        "tamaño no positivo en bloque {}",
                        block.id
                    )));
                }
            }
            match &block.content {
                BlockContent::Text(text) => {
                    for paragraph in &text.paragraphs {
                        for span in &paragraph.spans {
                            for mark in &span.marks {
                                if !TEXT_MARKS.contains(&mark.as_str()) {
                                    // Las marcas futuras se conservan, pero se
                                    // avisa mediante el prefijo desconocido.
                                    if !mark.starts_with("x-") {
                                        return Err(FolioError::Invalid(format!(
                                            "marca desconocida '{mark}' en bloque {}",
                                            block.id
                                        )));
                                    }
                                }
                            }
                            if let Some(link) = &span.link {
                                validate_url(link)?;
                            }
                        }
                    }
                }
                BlockContent::Table(table) => {
                    if table.rows == 0 || table.cols == 0 {
                        return Err(FolioError::Invalid(format!(
                            "tabla vacía en bloque {}",
                            block.id
                        )));
                    }
                    if table.rows.saturating_mul(table.cols) > 100_000 {
                        return Err(FolioError::Invalid("tabla demasiado grande".into()));
                    }
                    for cell in &table.cells {
                        if cell.row >= table.rows || cell.col >= table.cols {
                            return Err(FolioError::Invalid(format!(
                                "celda fuera de rango en bloque {}",
                                block.id
                            )));
                        }
                    }
                }
                BlockContent::Image(image) => {
                    validate_asset(&image.asset, &block.id)?;
                }
                BlockContent::File(file) => validate_asset(&file.asset, &block.id)?,
                BlockContent::Audio(audio) => validate_asset(&audio.asset, &block.id)?,
                BlockContent::Video(video) => validate_asset(&video.asset, &block.id)?,
                BlockContent::Web(web) => validate_url(&web.url)?,
                BlockContent::Drawing(drawing) => {
                    for stroke in &drawing.strokes {
                        if !stroke.width.is_finite() || stroke.width < 0.0 {
                            return Err(FolioError::Invalid("grosor de trazo inválido".into()));
                        }
                        for point in &stroke.points {
                            if point.iter().any(|value| !value.is_finite()) {
                                return Err(FolioError::Invalid(
                                    "punto de dibujo no finito".into(),
                                ));
                            }
                        }
                    }
                }
                BlockContent::Plugin(plugin) => {
                    if plugin.plugin.trim().is_empty() {
                        return Err(FolioError::Invalid("bloque de plugin sin plugin".into()));
                    }
                    let size = serde_json::to_vec(&plugin.state)
                        .map(|bytes| bytes.len())
                        .unwrap_or(usize::MAX);
                    if size > MAX_PLUGIN_STATE_BYTES {
                        return Err(FolioError::Invalid(format!(
                            "estado de plugin de {size} bytes supera 1 MB"
                        )));
                    }
                }
                BlockContent::Raw(_) => {}
            }
        }

        let size = serde_json::to_vec(self)?.len();
        if size > MAX_NOTE_BYTES {
            return Err(FolioError::Invalid(format!(
                "nota de {size} bytes supera el límite de 50 MB"
            )));
        }
        Ok(())
    }
}

fn validate_asset(asset: &str, block_id: &str) -> Result<(), FolioError> {
    if asset.is_empty() {
        return Err(FolioError::Invalid(format!("bloque {block_id} sin asset")));
    }
    if asset.len() != 64 || !asset.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(FolioError::Invalid(format!(
            "asset de bloque {block_id} no es BLAKE3 hex"
        )));
    }
    Ok(())
}

pub fn validate_url(url: &str) -> Result<(), FolioError> {
    let lower = url.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return Err(FolioError::Invalid("URL vacía".into()));
    }
    let allowed = ["http://", "https://", "mailto:", "folio://"];
    if !allowed.iter().any(|scheme| lower.starts_with(scheme)) {
        return Err(FolioError::Invalid(format!(
            "esquema de URL no permitido: {url}"
        )));
    }
    if lower.contains('\n') || lower.contains('\r') {
        return Err(FolioError::Invalid("URL con saltos de línea".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;
    use crate::model::{Block, Placement};

    fn note() -> Note {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        Note::new("T", &clock)
    }

    #[test]
    fn valid_note_passes() {
        let mut note = note();
        note.blocks.push(Block::text_plain("hola"));
        note.validate().unwrap();
    }

    #[test]
    fn duplicate_block_id_fails() {
        let mut note = note();
        let block = Block::text_plain("a");
        note.blocks.push(block.clone());
        note.blocks.push(block);
        assert!(note.validate().is_err());
    }

    #[test]
    fn invalid_free_size_fails() {
        let mut note = note();
        let mut block = Block::text_plain("a");
        block.placement = Placement::free(0.0, 0.0, 0.0, 10.0);
        note.blocks.push(block);
        assert!(note.validate().is_err());
    }

    #[test]
    fn bad_url_and_mailto() {
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("mailto:a@b.c").is_ok());
        assert!(validate_url("folio://note/x").is_ok());
    }

    #[test]
    fn future_unknown_mark_is_rejected_unless_namespaced() {
        let mut note = note();
        let mut paragraph = crate::model::Paragraph::body("x");
        paragraph.spans[0].marks.push("blink".into());
        note.blocks.push(Block::text(vec![paragraph]));
        assert!(note.validate().is_err());
    }
}
