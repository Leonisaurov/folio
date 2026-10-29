use std::collections::BTreeMap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::clock::Clock;
use crate::ids::new_id;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_TITLE_CHARS: usize = 500;
pub const MAX_BLOCKS: usize = 5_000;
pub const MAX_NOTE_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_PLUGIN_STATE_BYTES: usize = 1024 * 1024;

fn default_schema() -> u32 {
    SCHEMA_VERSION
}

fn is_zero_i64(value: &i64) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    #[serde(default = "default_schema")]
    pub schema: u32,
    pub id: String,
    #[serde(default)]
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub layout: NoteLayout,
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub meta: NoteMeta,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

impl Note {
    pub fn new(title: impl Into<String>, clock: &dyn Clock) -> Self {
        let now = clock.now_rfc3339();
        Self {
            schema: SCHEMA_VERSION,
            id: new_id(),
            title: title.into(),
            created_at: now.clone(),
            updated_at: now,
            layout: NoteLayout::default(),
            blocks: Vec::new(),
            meta: NoteMeta::default(),
            extra: Map::new(),
        }
    }

    pub fn touch(&mut self, clock: &dyn Clock) {
        self.updated_at = clock.now_rfc3339();
        self.meta.word_count = self.word_count();
    }

    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.clone();
        }
        self.title_from_first_line()
            .unwrap_or_else(|| "Nota sin título".to_string())
    }

    pub fn title_from_first_line(&self) -> Option<String> {
        for block in &self.blocks {
            let text = block.plain_text();
            let line = text.lines().find(|line| !line.trim().is_empty())?;
            let line = line.trim();
            if !line.is_empty() {
                let mut title: String = line.chars().take(120).collect();
                if line.chars().count() > 120 {
                    title.push('…');
                }
                return Some(title);
            }
        }
        None
    }

    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .map(Block::plain_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn word_count(&self) -> usize {
        self.plain_text().split_whitespace().count()
    }

    pub fn has_media(&self) -> bool {
        self.blocks.iter().any(|block| {
            matches!(
                block.content,
                BlockContent::Image(_)
                    | BlockContent::Audio(_)
                    | BlockContent::Video(_)
                    | BlockContent::File(_)
            )
        })
    }

    pub fn block(&self, id: &str) -> Option<&Block> {
        self.blocks.iter().find(|block| block.id == id)
    }

    pub fn block_mut(&mut self, id: &str) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|block| block.id == id)
    }

    /// Ordena visualmente los bloques y los devuelve al flujo.
    pub fn recompose(&mut self) {
        let mut indexed: Vec<(usize, Block)> = self.blocks.drain(..).enumerate().collect();
        indexed.sort_by(|(ia, a), (ib, b)| {
            let ka = a.placement.visual_key(*ia);
            let kb = b.placement.visual_key(*ib);
            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
        });
        self.blocks = indexed
            .into_iter()
            .map(|(_, mut block)| {
                block.placement = Placement::Flow;
                block
            })
            .collect();
    }

    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, crate::error::FolioError> {
        let note: Note = serde_json::from_slice(bytes)?;
        Ok(note)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteLayout {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_flow_width")]
    pub flow_width: f64,
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default)]
    pub show_grid: bool,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

fn default_mode() -> String {
    "flow".into()
}
fn default_flow_width() -> f64 {
    720.0
}
fn default_background() -> String {
    "paper".into()
}

impl Default for NoteLayout {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            flow_width: default_flow_width(),
            background: default_background(),
            show_grid: false,
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NoteMeta {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub word_count: usize,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Placement {
    #[default]
    Flow,
    Free {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        #[serde(default, skip_serializing_if = "is_zero_i64")]
        z: i64,
        #[serde(default, flatten)]
        extra: Map<String, Value>,
    },
}

impl Placement {
    pub fn free(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self::Free {
            x,
            y,
            w,
            h,
            z: 0,
            extra: Map::new(),
        }
    }

    pub fn is_free(&self) -> bool {
        matches!(self, Self::Free { .. })
    }

    pub fn with_z(self, new_z: i64) -> Placement {
        match self {
            Placement::Free {
                x, y, w, h, extra, ..
            } => Placement::Free {
                x,
                y,
                w,
                h,
                z: new_z,
                extra,
            },
            other => other,
        }
    }

    pub fn rect(&self) -> Option<(f64, f64, f64, f64, i64)> {
        match self {
            Self::Flow => None,
            Self::Free { x, y, w, h, z, .. } => Some((*x, *y, *w, *h, *z)),
        }
    }

    fn visual_key(&self, fallback_index: usize) -> (f64, f64) {
        match self {
            Self::Flow => (fallback_index as f64 * 1000.0, 0.0),
            Self::Free { x, y, .. } => (*y, *x),
        }
    }
}

/// Bloque con contenido tipado. Lo desconocido se conserva como `Raw`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Block {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub placement: Placement,
    pub content: BlockContent,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum BlockContent {
    Text(TextContent),
    Table(TableContent),
    Image(ImageContent),
    File(FileContent),
    Audio(AudioContent),
    Video(VideoContent),
    Web(WebContent),
    Drawing(DrawingContent),
    Plugin(PluginContent),
    Raw(Value),
}

#[derive(Deserialize)]
struct RawBlock {
    #[serde(default)]
    id: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    placement: Placement,
    #[serde(default)]
    content: Value,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl<'de> Deserialize<'de> for Block {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawBlock::deserialize(deserializer)?;
        let content = parse_content(&raw.kind, raw.content);
        Ok(Block {
            id: raw.id,
            kind: raw.kind,
            placement: raw.placement,
            content,
            extra: raw.extra,
        })
    }
}

fn parse_content(kind: &str, value: Value) -> BlockContent {
    macro_rules! parse {
        ($ty:ty, $variant:ident) => {
            match serde_json::from_value::<$ty>(value.clone()) {
                Ok(content) => return BlockContent::$variant(content),
                Err(_) => return BlockContent::Raw(value),
            }
        };
    }
    match kind {
        "text" => parse!(TextContent, Text),
        "table" => parse!(TableContent, Table),
        "image" => parse!(ImageContent, Image),
        "file" => parse!(FileContent, File),
        "audio" => parse!(AudioContent, Audio),
        "video" => parse!(VideoContent, Video),
        "web" => parse!(WebContent, Web),
        "drawing" => parse!(DrawingContent, Drawing),
        "plugin" => parse!(PluginContent, Plugin),
        _ => BlockContent::Raw(value),
    }
}

impl Block {
    pub fn new(kind: impl Into<String>, content: BlockContent) -> Self {
        Self {
            id: new_id(),
            kind: kind.into(),
            placement: Placement::Flow,
            content,
            extra: Map::new(),
        }
    }

    pub fn text(paragraphs: Vec<Paragraph>) -> Self {
        Self::new(
            "text",
            BlockContent::Text(TextContent {
                paragraphs,
                ..Default::default()
            }),
        )
    }

    pub fn text_plain(text: &str) -> Self {
        Self::text(vec![Paragraph::body(text)])
    }

    pub fn plain_text(&self) -> String {
        match &self.content {
            BlockContent::Text(text) => text.plain_text(),
            BlockContent::Table(table) => table.plain_text(),
            BlockContent::Image(image) => image.caption.clone(),
            BlockContent::File(file) => file.name.clone(),
            BlockContent::Audio(audio) => audio.title.clone(),
            BlockContent::Video(video) => video.title.clone(),
            BlockContent::Web(web) => {
                if web.title.is_empty() {
                    web.url.clone()
                } else {
                    format!("{} {}", web.title, web.description)
                }
            }
            BlockContent::Drawing(_) => String::new(),
            BlockContent::Plugin(plugin) => plugin
                .state
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&plugin.plugin)
                .to_string(),
            BlockContent::Raw(value) => value
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }
    }

    pub fn text_content(&self) -> Option<&TextContent> {
        match &self.content {
            BlockContent::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        self.text_content().map(TextContent::plain_text_ref)
    }

    pub fn is_preserved_unknown(&self) -> bool {
        matches!(self.content, BlockContent::Raw(_))
            || !matches!(
                self.kind.as_str(),
                "text"
                    | "table"
                    | "image"
                    | "file"
                    | "audio"
                    | "video"
                    | "web"
                    | "drawing"
                    | "plugin"
            )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextContent {
    #[serde(default)]
    pub paragraphs: Vec<Paragraph>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

impl TextContent {
    pub fn plain_text(&self) -> String {
        self.paragraphs
            .iter()
            .map(Paragraph::plain_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn plain_text_ref(&self) -> &str {
        // El texto de un span ya es el contenido; se expone el primero para
        // llamadas de conveniencia y se usa `plain_text` para el documento.
        self.paragraphs
            .first()
            .and_then(|paragraph| paragraph.spans.first())
            .map(|span| span.text.as_str())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Paragraph {
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default = "default_align")]
    pub align: String,
    #[serde(default)]
    pub indent: u8,
    #[serde(default)]
    pub checked: Option<bool>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub spans: Vec<Span>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

fn default_style() -> String {
    "body".into()
}
fn default_align() -> String {
    "left".into()
}

impl Paragraph {
    pub fn body(text: &str) -> Self {
        Self {
            style: "body".into(),
            align: "left".into(),
            indent: 0,
            checked: None,
            lang: None,
            spans: vec![Span::plain(text)],
            extra: Map::new(),
        }
    }

    pub fn styled(style: &str, text: &str) -> Self {
        let mut paragraph = Self::body(text);
        paragraph.style = style.into();
        paragraph
    }

    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Span {
    pub text: String,
    #[serde(default)]
    pub marks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

impl Span {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            marks: Vec::new(),
            link: None,
            color: None,
            extra: Map::new(),
        }
    }

    pub fn marked(text: impl Into<String>, marks: &[&str]) -> Self {
        Self {
            text: text.into(),
            marks: marks.iter().map(|mark| (*mark).to_string()).collect(),
            link: None,
            color: None,
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableContent {
    #[serde(default)]
    pub rows: usize,
    #[serde(default)]
    pub cols: usize,
    #[serde(default)]
    pub header: bool,
    #[serde(default)]
    pub widths: Vec<f64>,
    #[serde(default)]
    pub cells: Vec<TableCell>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

impl TableContent {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            header: true,
            widths: vec![180.0; cols],
            cells: Vec::new(),
            extra: Map::new(),
        }
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<&TableCell> {
        self.cells
            .iter()
            .find(|cell| cell.row == row && cell.col == col)
    }

    pub fn set_cell(&mut self, row: usize, col: usize, text: &str) {
        if row >= self.rows || col >= self.cols {
            return;
        }
        if let Some(cell) = self
            .cells
            .iter_mut()
            .find(|cell| cell.row == row && cell.col == col)
        {
            cell.paragraphs = vec![Paragraph::body(text)];
        } else {
            self.cells.push(TableCell {
                row,
                col,
                paragraphs: vec![Paragraph::body(text)],
                extra: Map::new(),
            });
        }
    }

    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                if let Some(cell) = self.cell(row, col) {
                    out.push_str(&cell.text());
                    out.push('\t');
                }
            }
            out.push('\n');
        }
        out
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    pub row: usize,
    pub col: usize,
    #[serde(default)]
    pub paragraphs: Vec<Paragraph>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

impl TableCell {
    pub fn text(&self) -> String {
        self.paragraphs
            .iter()
            .map(Paragraph::plain_text)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImageContent {
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub caption: String,
    #[serde(default)]
    pub alt: String,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_path: Option<String>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FileContent {
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mime: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AudioContent {
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peaks: Option<String>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VideoContent {
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WebContent {
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_web_mode")]
    pub mode: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

fn default_web_mode() -> String {
    "card".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DrawingContent {
    #[serde(default = "default_schema")]
    pub version: u32,
    #[serde(default = "default_drawing_width")]
    pub width: f64,
    #[serde(default = "default_drawing_height")]
    pub height: f64,
    #[serde(default = "default_background_transparent")]
    pub background: String,
    #[serde(default)]
    pub strokes: Vec<Stroke>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

fn default_drawing_width() -> f64 {
    1200.0
}
fn default_drawing_height() -> f64 {
    800.0
}
fn default_background_transparent() -> String {
    "transparent".into()
}

impl Default for DrawingContent {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            width: default_drawing_width(),
            height: default_drawing_height(),
            background: default_background_transparent(),
            strokes: Vec::new(),
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Stroke {
    #[serde(default = "default_pen")]
    pub tool: String,
    #[serde(default = "default_ink")]
    pub color: String,
    #[serde(default = "default_stroke_width")]
    pub width: f64,
    /// `[x, y, presión, tiempo_ms]`; la presión y el tiempo son opcionales.
    #[serde(default)]
    pub points: Vec<Vec<f64>>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

fn default_pen() -> String {
    "pen".into()
}
fn default_ink() -> String {
    "#1a1a1a".into()
}
fn default_stroke_width() -> f64 {
    2.5
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginContent {
    #[serde(default)]
    pub plugin: String,
    #[serde(default)]
    pub block: String,
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub state: Value,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default, flatten)]
    pub extra: Map<String, Value>,
}

impl Default for PluginContent {
    fn default() -> Self {
        Self {
            plugin: String::new(),
            block: String::new(),
            schema: SCHEMA_VERSION,
            state: Value::Object(Map::new()),
            height: None,
            extra: Map::new(),
        }
    }
}

/// Índice de nombres de estilo admitidos; los desconocidos se conservan.
pub const PARAGRAPH_STYLES: &[&str] = &[
    "body",
    "h1",
    "h2",
    "h3",
    "quote",
    "code",
    "list_item",
    "checklist_item",
];
pub const TEXT_MARKS: &[&str] = &[
    "bold",
    "italic",
    "underline",
    "strikethrough",
    "code",
    "highlight",
    "superscript",
    "subscript",
];
pub const KNOWN_BLOCK_KINDS: &[&str] = &[
    "text", "table", "image", "file", "audio", "video", "web", "drawing", "plugin",
];

/// Agrupa assets referenciados por la nota (para refcount y GC).
pub fn referenced_assets(note: &Note) -> Vec<String> {
    let mut assets: HashMap<String, ()> = HashMap::new();
    for block in &note.blocks {
        match &block.content {
            BlockContent::Image(image) if !image.asset.is_empty() => {
                assets.insert(image.asset.clone(), ());
            }
            BlockContent::File(file) if !file.asset.is_empty() => {
                assets.insert(file.asset.clone(), ());
            }
            BlockContent::Audio(audio) if !audio.asset.is_empty() => {
                assets.insert(audio.asset.clone(), ());
            }
            BlockContent::Video(video) if !video.asset.is_empty() => {
                assets.insert(video.asset.clone(), ());
                if let Some(poster) = &video.poster {
                    assets.insert(poster.clone(), ());
                }
            }
            BlockContent::Web(web) => {
                for hash in [web.favicon.as_ref(), web.image.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    assets.insert(hash.clone(), ());
                }
            }
            _ => {}
        }
    }
    let mut assets: Vec<String> = assets.into_keys().collect();
    assets.sort();
    assets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;

    fn clock() -> FixedClock {
        FixedClock::at("2026-10-29T09:00:00Z")
    }

    #[test]
    fn new_note_has_schema_and_id() {
        let note = Note::new("Hola", &clock());
        assert_eq!(note.schema, 1);
        assert!(!note.id.is_empty());
        assert_eq!(note.title, "Hola");
        assert_eq!(note.created_at, note.updated_at);
    }

    #[test]
    fn unknown_fields_survive_roundtrip() {
        let json = r#"{
            "schema": 1,
            "id": "01J9Z6Q3K8V5N2M1P0R7T4W6X9",
            "title": "T",
            "created_at": "2026-10-29T09:10:00Z",
            "updated_at": "2026-10-29T09:42:11Z",
            "futuro": {"a": 1},
            "blocks": [{
                "id": "b1",
                "type": "text",
                "placement": {"kind": "flow"},
                "content": {"paragraphs": []},
                "extra_block": true
            }]
        }"#;
        let note: Note = serde_json::from_str(json).unwrap();
        assert_eq!(note.extra["futuro"]["a"], 1);
        assert_eq!(note.blocks[0].extra["extra_block"], true);
        let out = serde_json::to_string(&note).unwrap();
        assert!(out.contains("futuro"));
        assert!(out.contains("extra_block"));
    }

    #[test]
    fn unknown_block_is_preserved() {
        let json = r#"{
            "id": "b1", "type": "callout",
            "placement": {"kind": "free", "x": 1.0, "y": 2.0, "w": 3.0, "h": 4.0},
            "content": {"text": "futuro", "color": "red"}
        }"#;
        let block: Block = serde_json::from_str(json).unwrap();
        assert!(block.is_preserved_unknown());
        assert_eq!(block.kind, "callout");
        let again = serde_json::to_value(&block).unwrap();
        assert_eq!(again["content"]["color"], "red");
        assert_eq!(again["placement"]["x"], 1.0);
    }

    #[test]
    fn title_derives_from_first_line() {
        let mut note = Note::new("", &clock());
        note.blocks
            .push(Block::text_plain("\n  Primera línea\nSegunda"));
        assert_eq!(note.display_title(), "Primera línea");
    }

    #[test]
    fn recompose_orders_free_blocks_and_clears_placement() {
        let mut note = Note::new("", &clock());
        let mut a = Block::text_plain("a");
        a.placement = Placement::free(10.0, 200.0, 100.0, 20.0);
        let mut b = Block::text_plain("b");
        b.placement = Placement::free(10.0, 10.0, 100.0, 20.0);
        let c = Block::text_plain("c");
        note.blocks = vec![a, b, c];
        note.recompose();
        assert_eq!(note.blocks[0].plain_text(), "b");
        assert!(note.blocks.iter().all(|block| !block.placement.is_free()));
    }

    #[test]
    fn table_roundtrip_and_text() {
        let mut table = TableContent::new(2, 2);
        table.set_cell(0, 0, "Nombre");
        table.set_cell(1, 1, "Valor");
        let block = Block::new("table", BlockContent::Table(table));
        let encoded = serde_json::to_string(&block).unwrap();
        let decoded: Block = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.plain_text().contains("Nombre"));
        assert!(decoded.plain_text().contains("Valor"));
    }
}
