use crate::clock::Clock;
use crate::error::FolioError;
use crate::model::{Block, BlockContent, Note, NoteLayout, Placement, TextContent};

/// Comandos de dominio sobre una nota. Toda mutación pasa por aquí.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum NoteCommand {
    InsertBlock {
        block: Block,
        index: Option<usize>,
    },
    RemoveBlock {
        id: String,
    },
    UpdateText {
        block_id: String,
        content: TextContent,
    },
    SetTitle {
        title: String,
    },
    MoveBlock {
        id: String,
        placement: Placement,
    },
    SetLayout {
        layout: NoteLayout,
    },
    DuplicateBlock {
        id: String,
    },
    Recompose,
}

impl NoteCommand {
    pub fn apply(&self, note: &mut Note, clock: &dyn Clock) -> Result<(), FolioError> {
        match self {
            Self::InsertBlock { block, index } => {
                if note.blocks.iter().any(|existing| existing.id == block.id) {
                    return Err(FolioError::Conflict(format!(
                        "ya existe el bloque {}",
                        block.id
                    )));
                }
                let index = index.unwrap_or(note.blocks.len()).min(note.blocks.len());
                note.blocks.insert(index, block.clone());
            }
            Self::RemoveBlock { id } => {
                let before = note.blocks.len();
                note.blocks.retain(|block| &block.id != id);
                if note.blocks.len() == before {
                    return Err(FolioError::NotFound(format!("bloque {id}")));
                }
            }
            Self::UpdateText { block_id, content } => {
                let block = note
                    .block_mut(block_id)
                    .ok_or_else(|| FolioError::NotFound(format!("bloque {block_id}")))?;
                block.content = BlockContent::Text(content.clone());
            }
            Self::SetTitle { title } => {
                note.title = title.clone();
            }
            Self::MoveBlock { id, placement } => {
                let block = note
                    .block_mut(id)
                    .ok_or_else(|| FolioError::NotFound(format!("bloque {id}")))?;
                block.placement = placement.clone();
            }
            Self::SetLayout { layout } => {
                note.layout = layout.clone();
            }
            Self::DuplicateBlock { id } => {
                let block = note
                    .block(id)
                    .ok_or_else(|| FolioError::NotFound(format!("bloque {id}")))?
                    .clone();
                let mut copy = block;
                copy.id = crate::ids::new_id();
                if let Placement::Free { x, y, .. } = &mut copy.placement {
                    *x += 24.0;
                    *y += 24.0;
                }
                note.blocks.push(copy);
            }
            Self::Recompose => note.recompose(),
        }
        note.touch(clock);
        note.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;
    use crate::model::{Paragraph, Placement};

    fn setup() -> (Note, FixedClock) {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        (Note::new("", &clock), clock)
    }

    #[test]
    fn insert_update_remove() {
        let (mut note, clock) = setup();
        let block = Block::text_plain("hola");
        let id = block.id.clone();
        NoteCommand::InsertBlock { block, index: None }
            .apply(&mut note, &clock)
            .unwrap();
        let content = TextContent {
            paragraphs: vec![Paragraph::body("adiós")],
            ..Default::default()
        };
        NoteCommand::UpdateText {
            block_id: id.clone(),
            content,
        }
        .apply(&mut note, &clock)
        .unwrap();
        assert_eq!(note.plain_text(), "adiós");
        NoteCommand::RemoveBlock { id: id.clone() }
            .apply(&mut note, &clock)
            .unwrap();
        assert!(note.blocks.is_empty());
        assert!(
            NoteCommand::RemoveBlock { id }
                .apply(&mut note, &clock)
                .is_err()
        );
    }

    #[test]
    fn move_and_recompose() {
        let (mut note, clock) = setup();
        let block = Block::text_plain("hola");
        let id = block.id.clone();
        NoteCommand::InsertBlock { block, index: None }
            .apply(&mut note, &clock)
            .unwrap();
        NoteCommand::MoveBlock {
            id: id.clone(),
            placement: Placement::free(100.0, 50.0, 300.0, 80.0),
        }
        .apply(&mut note, &clock)
        .unwrap();
        assert!(note.block(&id).unwrap().placement.is_free());
        NoteCommand::Recompose.apply(&mut note, &clock).unwrap();
        assert!(!note.block(&id).unwrap().placement.is_free());
    }

    #[test]
    fn duplication_offsets_free_placement() {
        let (mut note, clock) = setup();
        let mut block = Block::text_plain("hola");
        block.placement = Placement::free(10.0, 10.0, 100.0, 30.0);
        let id = block.id.clone();
        note.blocks.push(block);
        NoteCommand::DuplicateBlock { id: id.clone() }
            .apply(&mut note, &clock)
            .unwrap();
        assert_eq!(note.blocks.len(), 2);
        let (x, y, ..) = note.blocks[1].placement.rect().unwrap();
        assert_eq!((x, y), (34.0, 34.0));
    }
}
