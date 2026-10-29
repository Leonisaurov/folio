//! Geometría pura del lienzo: flujo, bloques libres, viewport, hit-testing,
//! guías y recomposición. Sin GTK.

use std::collections::HashMap;

use folio_core::{Block, Note, Placement};
use serde::{Deserialize, Serialize};

pub const DEFAULT_GAP: f64 = 16.0;
pub const MIN_ZOOM: f64 = 0.25;
pub const MAX_ZOOM: f64 = 4.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.right()
            && point.y <= self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }

    pub fn translated(&self, dx: f64, dy: f64) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    pub fn inset(&self, amount: f64) -> Rect {
        Rect::new(
            self.x + amount,
            self.y + amount,
            (self.w - amount * 2.0).max(0.0),
            (self.h - amount * 2.0).max(0.0),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub scale: f64,
    pub offset: Point,
    pub size: Size,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: Point::default(),
            size: Size::new(1000.0, 700.0),
        }
    }
}

impl Viewport {
    pub fn new(size: Size) -> Self {
        Self {
            scale: 1.0,
            offset: Point::default(),
            size,
        }
    }

    pub fn clamp_scale(scale: f64) -> f64 {
        scale.clamp(MIN_ZOOM, MAX_ZOOM)
    }

    pub fn screen_from_world(&self, point: Point) -> Point {
        Point::new(
            (point.x - self.offset.x) * self.scale,
            (point.y - self.offset.y) * self.scale,
        )
    }

    pub fn world_from_screen(&self, point: Point) -> Point {
        Point::new(
            point.x / self.scale + self.offset.x,
            point.y / self.scale + self.offset.y,
        )
    }

    pub fn visible_world(&self) -> Rect {
        let top_left = self.world_from_screen(Point::default());
        Rect::new(
            top_left.x,
            top_left.y,
            self.size.width / self.scale,
            self.size.height / self.scale,
        )
    }

    /// Zoom manteniendo fijo el punto bajo el cursor.
    pub fn zoom_at(&mut self, screen_point: Point, new_scale: f64) {
        let anchor = self.world_from_screen(screen_point);
        self.scale = Self::clamp_scale(new_scale);
        self.offset = Point::new(
            anchor.x - screen_point.x / self.scale,
            anchor.y - screen_point.y / self.scale,
        );
    }

    pub fn pan(&mut self, dx_screen: f64, dy_screen: f64) {
        self.offset.x -= dx_screen / self.scale;
        self.offset.y -= dy_screen / self.scale;
    }

    /// Encuadra un rectángulo del mundo con margen, sin superar 100 %.
    pub fn fit(&mut self, rect: Rect, padding: f64) {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }
        let available_w = (self.size.width - padding * 2.0).max(1.0);
        let available_h = (self.size.height - padding * 2.0).max(1.0);
        self.scale = Self::clamp_scale((available_w / rect.w).min(available_h / rect.h).min(1.0));
        self.offset = Point::new(
            rect.center().x - self.size.width / (2.0 * self.scale),
            rect.center().y - self.size.height / (2.0 * self.scale),
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockBox {
    pub id: String,
    pub rect: Rect,
    pub z: i64,
    pub free: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub boxes: Vec<BlockBox>,
    by_id: HashMap<String, Rect>,
    pub bbox: Rect,
    pub flow_width: f64,
    pub flow_height: f64,
}

impl Layout {
    pub fn rect(&self, block_id: &str) -> Option<Rect> {
        self.by_id.get(block_id).copied()
    }

    pub fn flow_bbox(&self) -> Rect {
        Rect::new(0.0, 0.0, self.flow_width, self.flow_height)
    }

    /// Hit-testing con orden Z; devuelve el bloque superior.
    pub fn hit_test(&self, world: Point) -> Option<&BlockBox> {
        self.boxes
            .iter()
            .filter(|block| block.rect.contains(world))
            .max_by_key(|block| block.z)
    }

    /// Orden visual (y, x) para «Recomponer».
    pub fn recompose_order(&self) -> Vec<String> {
        let mut boxes = self.boxes.clone();
        boxes.sort_by(|a, b| {
            a.rect
                .y
                .partial_cmp(&b.rect.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.rect
                        .x
                        .partial_cmp(&b.rect.x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        boxes.into_iter().map(|block| block.id).collect()
    }

    /// Bloques visibles en el viewport con margen de pantallas.
    pub fn visible(&self, viewport: &Viewport, margin_screens: f64) -> Vec<&BlockBox> {
        let mut visible = viewport.visible_world();
        let dx = visible.w * margin_screens;
        let dy = visible.h * margin_screens;
        visible = Rect::new(
            visible.x - dx,
            visible.y - dy,
            visible.w + dx * 2.0,
            visible.h + dy * 2.0,
        );
        self.boxes
            .iter()
            .filter(|block| block.rect.intersects(&visible))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FlowOptions {
    pub gap: f64,
    pub top: f64,
    pub left: f64,
}

/// Calcula el layout de flujo y los rectángulos libres.
pub fn layout_note<F>(note: &Note, mut measure: F) -> Layout
where
    F: FnMut(&Block, f64) -> f64,
{
    layout_note_with(
        note,
        FlowOptions {
            gap: DEFAULT_GAP,
            top: 0.0,
            left: 0.0,
        },
        &mut measure,
    )
}

pub fn layout_note_with<F>(note: &Note, options: FlowOptions, measure: &mut F) -> Layout
where
    F: FnMut(&Block, f64) -> f64,
{
    let flow_width = note.layout.flow_width.max(80.0);
    let mut y = options.top;
    let mut boxes = Vec::new();
    let mut by_id = HashMap::new();
    let mut bbox: Option<Rect> = None;
    for block in &note.blocks {
        let rect = match &block.placement {
            Placement::Flow => {
                let height = measure(block, flow_width).max(1.0);
                let rect = Rect::new(options.left, y, flow_width, height);
                y += height + options.gap;
                rect
            }
            Placement::Free { x, y, w, h, .. } => Rect::new(*x, *y, *w, *h),
        };
        let z = block.placement.rect().map(|(_, _, _, _, z)| z).unwrap_or(0);
        boxes.push(BlockBox {
            id: block.id.clone(),
            rect,
            z,
            free: block.placement.is_free(),
        });
        by_id.insert(block.id.clone(), rect);
        bbox = Some(match bbox {
            Some(current) => current.union(&rect),
            None => rect,
        });
    }
    let flow_height = (y - options.gap - options.top).max(0.0);
    Layout {
        boxes,
        by_id,
        bbox: bbox.unwrap_or(Rect::new(0.0, 0.0, flow_width, 0.0)),
        flow_width,
        flow_height,
    }
}

/// Ajusta un rectángulo a guías cercanas (otros bloques y centro del flujo).
pub fn snap_rect(rect: Rect, others: &[Rect], threshold: f64) -> Rect {
    let mut best_x = None;
    let mut best_y = None;
    for other in others {
        for (candidate, target) in [
            (rect.x, other.x),
            (rect.x, other.right() - rect.w),
            (rect.right(), other.right()),
            (rect.right(), other.x + rect.w),
        ] {
            let delta = target - candidate;
            if delta.abs() <= threshold
                && best_x
                    .map(|current: f64| delta.abs() < current.abs())
                    .unwrap_or(true)
            {
                best_x = Some(delta);
            }
        }
        for (candidate, target) in [
            (rect.y, other.y),
            (rect.y, other.bottom() - rect.h),
            (rect.bottom(), other.bottom()),
        ] {
            let delta = target - candidate;
            if delta.abs() <= threshold
                && best_y
                    .map(|current: f64| delta.abs() < current.abs())
                    .unwrap_or(true)
            {
                best_y = Some(delta);
            }
        }
    }
    Rect::new(
        rect.x + best_x.unwrap_or(0.0),
        rect.y + best_y.unwrap_or(0.0),
        rect.w,
        rect.h,
    )
}

/// Convierte una caída de pantalla en un `Placement::Free` del mundo.
pub fn placement_from_drop(
    viewport: &Viewport,
    drop_screen: Point,
    grab_offset: Point,
    size: Size,
    z: i64,
) -> Placement {
    let world = viewport.world_from_screen(drop_screen);
    Placement::free(
        world.x - grab_offset.x,
        world.y - grab_offset.y,
        size.width,
        size.height,
    )
    .with_z(z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use folio_core::{
        FixedClock, Note,
        model::{BlockContent, DrawingContent, TableContent},
    };

    fn note(blocks: usize) -> Note {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let mut note = Note::new("", &clock);
        for index in 0..blocks {
            let mut block = Block::text_plain(&format!("bloque {index}"));
            if index % 10 == 0 {
                block.placement =
                    Placement::free(800.0 + (index as f64), (index as f64) * 10.0, 220.0, 90.0);
            }
            note.blocks.push(block);
        }
        note
    }

    fn measure(block: &Block, width: f64) -> f64 {
        match &block.content {
            BlockContent::Text(text) => 20.0 * text.paragraphs.len() as f64 + width / 100.0,
            BlockContent::Table(table) => 24.0 * table.rows as f64,
            BlockContent::Drawing(_) => 300.0,
            _ => 180.0,
        }
    }

    #[test]
    fn flow_stacks_and_free_keeps_position() {
        let note = note(12);
        let layout = layout_note(&note, measure);
        let free = layout.rect(&note.blocks[0].id).unwrap();
        let flow_a = layout.rect(&note.blocks[1].id).unwrap();
        let flow_b = layout.rect(&note.blocks[2].id).unwrap();
        assert!(note.blocks[0].placement.is_free());
        assert_eq!(free.x, 800.0);
        assert_eq!(flow_a.x, 0.0);
        assert!(flow_b.y > flow_a.bottom());
    }

    #[test]
    fn bbox_contains_everything() {
        let note = note(50);
        let layout = layout_note(&note, measure);
        for block in &layout.boxes {
            assert!(layout.bbox.x <= block.rect.x);
            assert!(layout.bbox.y <= block.rect.y);
            assert!(layout.bbox.right() >= block.rect.right());
            assert!(layout.bbox.bottom() >= block.rect.bottom());
        }
    }

    #[test]
    fn hit_testing_at_all_zooms() {
        let note = note(10);
        let layout = layout_note(&note, measure);
        let target = &layout.boxes[3];
        for scale in [0.25, 1.0, 4.0] {
            let mut viewport = Viewport::new(Size::new(1200.0, 800.0));
            viewport.scale = scale;
            let screen = viewport.screen_from_world(target.rect.center());
            let world = viewport.world_from_screen(screen);
            let hit = layout.hit_test(world).unwrap();
            assert_eq!(hit.id, target.id);
        }
    }

    #[test]
    fn zoom_keeps_point_under_cursor() {
        let mut viewport = Viewport::default();
        let screen = Point::new(400.0, 300.0);
        let before = viewport.world_from_screen(screen);
        viewport.zoom_at(screen, 2.5);
        let after = viewport.world_from_screen(screen);
        assert!((before.x - after.x).abs() < 1e-9);
        assert!((before.y - after.y).abs() < 1e-9);
    }

    #[test]
    fn fit_document_and_all() {
        let note = note(20);
        let layout = layout_note(&note, measure);
        let mut viewport = Viewport::new(Size::new(1000.0, 700.0));
        viewport.fit(layout.flow_bbox(), 24.0);
        assert!(viewport.scale <= 1.0);
        assert!(viewport.scale >= MIN_ZOOM);
        viewport.fit(layout.bbox, 24.0);
        assert!(layout.bbox.w >= layout.flow_width);
    }

    #[test]
    fn recompose_order_is_visual_and_stable() {
        let note = note(10);
        let layout = layout_note(&note, measure);
        let order = layout.recompose_order();
        assert_eq!(order.len(), 10);
        let positions: Vec<Rect> = order.iter().map(|id| layout.rect(id).unwrap()).collect();
        for window in positions.windows(2) {
            assert!(window[0].y <= window[1].y);
        }
    }

    #[test]
    fn snapping_to_guides() {
        let rect = Rect::new(103.0, 50.0, 100.0, 40.0);
        let other = Rect::new(100.0, 200.0, 100.0, 40.0);
        let snapped = snap_rect(rect, &[other], 8.0);
        assert_eq!(snapped.x, 100.0);
        let far = snap_rect(Rect::new(300.0, 300.0, 10.0, 10.0), &[other], 8.0);
        assert_eq!(far.x, 300.0);
    }

    #[test]
    fn drop_converts_to_world_coordinates() {
        let mut viewport = Viewport::new(Size::new(1000.0, 700.0));
        viewport.offset = Point::new(20.0, 10.0);
        viewport.scale = 2.0;
        let placement = placement_from_drop(
            &viewport,
            Point::new(220.0, 210.0),
            Point::new(10.0, 10.0),
            Size::new(200.0, 100.0),
            7,
        );
        let (x, y, w, h, z) = placement.rect().unwrap();
        assert_eq!((x, y, w, h, z), (120.0, 105.0, 200.0, 100.0, 7));
    }

    #[test]
    fn five_hundred_blocks_layout_quickly() {
        let note = note(500);
        let start = std::time::Instant::now();
        let layout = layout_note(&note, measure);
        let elapsed = start.elapsed();
        assert_eq!(layout.boxes.len(), 500);
        assert!(
            elapsed.as_millis() < 100,
            "layout de 500 bloques: {elapsed:?}"
        );
        let visible = layout.visible(&Viewport::new(Size::new(1200.0, 800.0)), 2.0);
        assert!(!visible.is_empty());
        assert!(visible.len() < 500);
    }

    #[test]
    fn table_and_drawing_have_intrinsic_heights() {
        let clock = FixedClock::at("2026-10-29T09:00:00Z");
        let mut note = Note::new("", &clock);
        note.blocks.push(Block::new(
            "table",
            BlockContent::Table(TableContent::new(10, 4)),
        ));
        note.blocks.push(Block::new(
            "drawing",
            BlockContent::Drawing(DrawingContent::default()),
        ));
        let layout = layout_note(&note, measure);
        let table = layout.rect(&note.blocks[0].id).unwrap();
        let drawing = layout.rect(&note.blocks[1].id).unwrap();
        assert_eq!(table.h, 240.0);
        assert_eq!(drawing.h, 300.0);
    }
}
