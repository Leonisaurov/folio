//! Utilidades de texto compartidas por editor, exportadores e importadores.

use crate::model::{Block, BlockContent, Paragraph, Span, TableContent, TextContent};

pub fn paragraphs_from_plain(text: &str) -> Vec<Paragraph> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut paragraphs: Vec<Paragraph> = normalized
        .split('\n')
        .map(|line| {
            if let Some(rest) = line.strip_prefix("### ") {
                Paragraph::styled("h3", rest)
            } else if let Some(rest) = line.strip_prefix("## ") {
                Paragraph::styled("h2", rest)
            } else if let Some(rest) = line.strip_prefix("# ") {
                Paragraph::styled("h1", rest)
            } else if let Some(rest) = line.strip_prefix("> ") {
                Paragraph::styled("quote", rest)
            } else if let Some(rest) = line.strip_prefix("- [ ] ") {
                let mut paragraph = Paragraph::styled("checklist_item", rest);
                paragraph.checked = Some(false);
                paragraph
            } else if let Some(rest) = line.strip_prefix("- [x] ") {
                let mut paragraph = Paragraph::styled("checklist_item", rest);
                paragraph.checked = Some(true);
                paragraph
            } else if let Some(rest) = line.strip_prefix("- ") {
                Paragraph::styled("list_item", rest)
            } else {
                Paragraph::body(line)
            }
        })
        .collect();
    if paragraphs.is_empty() {
        paragraphs.push(Paragraph::body(""));
    }
    paragraphs
}

pub fn text_content_from_plain(text: &str) -> TextContent {
    TextContent {
        paragraphs: paragraphs_from_plain(text),
        ..Default::default()
    }
}

/// Interpreta **negrita**, *cursiva* y `código` en una línea.
pub fn spans_from_markdown(line: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut index = 0;
    let mut plain = String::new();
    while index < chars.len() {
        let rest = &chars[index..];
        let marker_len = if rest.starts_with(&['*', '*']) {
            Some((2, "bold"))
        } else if rest.starts_with(&['*']) {
            Some((1, "italic"))
        } else if rest.starts_with(&['`']) {
            Some((1, "code"))
        } else {
            None
        };
        if let Some((marker_len, mark)) = marker_len {
            let marker = if mark == "bold" {
                "**"
            } else if mark == "italic" {
                "*"
            } else {
                "`"
            };
            let after = &line[char_byte_index(line, index + marker_len)..];
            if let Some(end) = after.find(marker) {
                if !plain.is_empty() {
                    spans.push(Span::plain(std::mem::take(&mut plain)));
                }
                let text = &after[..end];
                if !text.is_empty() {
                    spans.push(Span::marked(text, &[mark]));
                }
                let advance = marker_len + text.chars().count() + marker_len;
                index += advance;
                continue;
            }
        }
        plain.push(chars[index]);
        index += 1;
    }
    if !plain.is_empty() {
        spans.push(Span::plain(plain));
    }
    if spans.is_empty() {
        spans.push(Span::plain(""));
    }
    spans
}

fn char_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

/// Convierte CSV/TSV pegado en una tabla; detecta el separador.
pub fn table_from_delimited(text: &str) -> Option<TableContent> {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.len() < 2 {
        return None;
    }
    let separator = if lines[0].contains('\t') {
        '\t'
    } else if lines[0].contains(',') {
        ','
    } else {
        return None;
    };
    let rows_data: Vec<Vec<&str>> = lines
        .iter()
        .map(|line| line.split(separator).map(str::trim).collect())
        .collect();
    let cols = rows_data.iter().map(Vec::len).max().unwrap_or(0);
    if cols == 0 {
        return None;
    }
    let mut table = TableContent::new(rows_data.len(), cols);
    for (row, cells) in rows_data.iter().enumerate() {
        for (col, text) in cells.iter().enumerate() {
            table.set_cell(row, col, text);
        }
    }
    Some(table)
}

/// Construye un bloque de texto a partir de texto plano.
pub fn text_block(text: &str) -> Block {
    Block::new("text", BlockContent::Text(text_content_from_plain(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_shortcuts_become_styles() {
        let paragraphs = paragraphs_from_plain("# Título\n- item\n- [x] hecho");
        assert_eq!(paragraphs[0].style, "h1");
        assert_eq!(paragraphs[1].style, "list_item");
        assert_eq!(paragraphs[2].checked, Some(true));
    }

    #[test]
    fn inline_marks() {
        let spans = spans_from_markdown("Hola **mundo** y *tú* con `code`");
        assert!(spans.iter().any(|span| span.marks.contains(&"bold".into())));
        assert!(
            spans
                .iter()
                .any(|span| span.marks.contains(&"italic".into()))
        );
        assert!(spans.iter().any(|span| span.marks.contains(&"code".into())));
        let joined: String = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(joined, "Hola mundo y tú con code");
    }

    #[test]
    fn csv_and_tsv() {
        let csv = table_from_delimited("a,b\n1,2\n").unwrap();
        assert_eq!((csv.rows, csv.cols), (2, 2));
        assert_eq!(csv.cell(1, 1).unwrap().text(), "2");
        let tsv = table_from_delimited("a\tb\n1\t2\n").unwrap();
        assert_eq!(tsv.cols, 2);
        assert!(table_from_delimited("solo una línea").is_none());
    }
}
