use crate::model::Note;

/// Pila de undo basada en instantáneas acotadas del documento.
///
/// Es simple, robusta y abarca texto, formato, canvas y DnD con una sola
/// implementación. El límite evita crecer sin control.
#[derive(Debug, Clone)]
pub struct UndoStack {
    undo: Vec<Note>,
    redo: Vec<Note>,
    limit: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(100)
    }
}

impl UndoStack {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    pub fn checkpoint(&mut self, note: &Note) {
        self.undo.push(note.clone());
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Devuelve el estado anterior y guarda el actual en redo.
    pub fn undo(&mut self, current: &Note) -> Option<Note> {
        let previous = self.undo.pop()?;
        self.redo.push(current.clone());
        if self.redo.len() > self.limit {
            self.redo.remove(0);
        }
        Some(previous)
    }

    /// Devuelve el estado siguiente y guarda el actual en undo.
    pub fn redo(&mut self, current: &Note) -> Option<Note> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        Some(next)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn depth(&self) -> usize {
        self.undo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;

    #[test]
    fn hundred_steps_roundtrip() {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let mut note = Note::new("t", &clock);
        let mut stack = UndoStack::new(100);
        for index in 0..100 {
            stack.checkpoint(&note);
            note.title = format!("paso {index}");
        }
        for _ in 0..100 {
            note = stack.undo(&note).unwrap();
        }
        assert_eq!(note.title, "t");
        for _ in 0..100 {
            note = stack.redo(&note).unwrap();
        }
        assert_eq!(note.title, "paso 99");
    }

    #[test]
    fn new_edit_clears_redo() {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let mut note = Note::new("t", &clock);
        let mut stack = UndoStack::default();
        stack.checkpoint(&note);
        note.title = "a".into();
        note = stack.undo(&note).unwrap();
        assert!(stack.can_redo());
        stack.checkpoint(&note);
        assert!(!stack.can_redo());
    }
}
