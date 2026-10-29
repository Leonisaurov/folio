use std::path::PathBuf;

use clap::{Parser, Subcommand};
use folio_core::{Block, FixedClock, Note, SystemClock, model::TableContent, text::text_block};
use folio_store::{Index, Store};

#[derive(Parser)]
#[command(
    name = "folio-tools",
    about = "Herramientas de desarrollo y bancos de Folio"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Genera una bóveda sintética para pruebas de rendimiento.
    GenVault {
        #[arg(long, default_value_t = 1000)]
        notes: usize,
        #[arg(long, default_value_t = 10)]
        books: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        index: bool,
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Genera una tabla grande y la imprime como Markdown.
    GenTable {
        #[arg(long, default_value_t = 100)]
        rows: usize,
        #[arg(long, default_value_t = 20)]
        cols: usize,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Ejecuta el layout de N bloques y muestra el tiempo.
    BenchLayout {
        #[arg(long, default_value_t = 500)]
        blocks: usize,
    },
    /// Genera una nota de ejemplo con todos los tipos de bloque.
    GenDemo {
        #[arg(long)]
        vault: PathBuf,
    },
    /// Genera una nota con N bloques para probar el lienzo.
    GenCanvas {
        #[arg(long, default_value_t = 500)]
        blocks: usize,
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::GenVault {
            notes,
            books,
            out,
            index,
            seed,
        } => gen_vault(&out, notes, books, index, seed)?,
        Command::GenTable { rows, cols, out } => {
            let markdown = table_markdown(rows, cols);
            match out {
                Some(path) => std::fs::write(path, markdown)?,
                None => print!("{markdown}"),
            }
        }
        Command::BenchLayout { blocks } => bench_layout(blocks),
        Command::GenDemo { vault } => gen_demo(&vault)?,
        Command::GenCanvas { blocks, out } => gen_canvas(&out, blocks)?,
    }
    Ok(())
}

fn gen_vault(
    out: &std::path::Path,
    notes: usize,
    books: usize,
    build_index: bool,
    seed: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    let clock = SystemClock;
    let store = Store::init(out, "Bóveda sintética", &clock)?;
    let books_count = books.max(1);
    let mut created_books = Vec::new();
    for index in 0..books_count {
        let title = format!("Libro {index:03}");
        let book = store.create_book(&title, "manual", None, None, &title, &clock)?;
        created_books.push(book);
    }
    let mut state: u64 = seed.unwrap_or(0x5eed);
    for index in 0..notes {
        let book = &created_books[index % created_books.len()];
        let title = format!("Nota sintética {index:05}");
        let mut note = store.create_note(book, &title, &clock)?;
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let words = 20 + (state % 180) as usize;
        let body = (0..words)
            .map(|word| format!("palabra{}", (word * 7 + index) % 997))
            .collect::<Vec<_>>()
            .join(" ");
        note.blocks.push(text_block(&body));
        if index % 50 == 0 {
            let mut table = TableContent::new(3, 3);
            for row in 0..3 {
                for col in 0..3 {
                    table.set_cell(row, col, &format!("{row},{col}"));
                }
            }
            note.blocks.push(Block::new(
                "table",
                folio_core::model::BlockContent::Table(table),
            ));
        }
        store.save_note(book, &mut note, &clock)?;
    }
    if build_index {
        let mut index = Index::open(&store.vault.index_path())?;
        let count = index.rebuild(&store)?;
        println!("indexadas {count} notas");
    }
    println!(
        "bóveda: {} · {} libros · {} notas · {:.2?}",
        out.display(),
        books_count,
        notes,
        start.elapsed()
    );
    Ok(())
}

fn table_markdown(rows: usize, cols: usize) -> String {
    let mut out = String::new();
    for col in 0..cols {
        out.push_str(&format!("| col{col} "));
    }
    out.push_str("|\n|");
    for _ in 0..cols {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in 0..rows {
        for col in 0..cols {
            out.push_str(&format!("| {row},{col} "));
        }
        out.push_str("|\n");
    }
    out
}

fn bench_layout(blocks: usize) {
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut note = Note::new("Bench", &clock);
    for index in 0..blocks {
        let mut block = Block::text_plain(&format!("bloque {index}"));
        if index % 10 == 0 {
            block.placement =
                folio_core::Placement::free(800.0 + index as f64, index as f64 * 10.0, 220.0, 90.0);
        }
        note.blocks.push(block);
    }
    let start = std::time::Instant::now();
    let layout = folio_layout::layout_note(&note, |block, width| match &block.content {
        folio_core::model::BlockContent::Text(text) => {
            20.0 * text.paragraphs.len() as f64 + width / 100.0
        }
        _ => 100.0,
    });
    let elapsed = start.elapsed();
    println!(
        "{} bloques · {:.2?} · bbox {:.0}x{:.0}",
        layout.boxes.len(),
        elapsed,
        layout.bbox.w,
        layout.bbox.h
    );
}

fn gen_demo(vault: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let clock = SystemClock;
    let store = Store::init(vault, "Demo", &clock)?;
    let book = store.create_book("Proyectos", "manual", None, None, "proyectos", &clock)?;
    let mut note = store.create_note(&book, "Demo de bloques", &clock)?;
    note.blocks.push(Block::text(vec![
        folio_core::model::Paragraph::styled("h1", "Demo de Folio"),
        folio_core::model::Paragraph::body("Texto con **negrita** y una lista."),
        folio_core::model::Paragraph::styled("list_item", "Primer elemento"),
        folio_core::model::Paragraph::styled("quote", "Una cita"),
    ]));
    let mut table = TableContent::new(3, 3);
    table.set_cell(0, 0, "Mes");
    table.set_cell(0, 1, "Ventas");
    table.set_cell(1, 0, "Ene");
    table.set_cell(1, 1, "30");
    table.set_cell(2, 0, "Feb");
    table.set_cell(2, 1, "70");
    note.blocks.push(Block::new(
        "table",
        folio_core::model::BlockContent::Table(table),
    ));
    note.blocks.push(Block::new(
        "drawing",
        folio_core::model::BlockContent::Drawing(folio_core::model::DrawingContent {
            strokes: vec![folio_core::model::Stroke {
                points: vec![vec![40.0, 40.0], vec![200.0, 120.0], vec![320.0, 60.0]],
                ..Default::default()
            }],
            ..Default::default()
        }),
    ));
    note.blocks.push(Block::new(
        "plugin",
        folio_core::model::BlockContent::Plugin(folio_core::model::PluginContent {
            plugin: "io.github.folio.progress".into(),
            block: "progress".into(),
            schema: 1,
            state: serde_json::json!({"value": 0.4, "target": 1.0, "label": "Demo"}),
            height: Some(140.0),
            ..Default::default()
        }),
    ));
    store.save_note(&book, &mut note, &clock)?;
    println!("{}", vault.display());
    Ok(())
}

fn gen_canvas(out: &std::path::Path, blocks: usize) -> Result<(), Box<dyn std::error::Error>> {
    let clock = SystemClock;
    let store = Store::init(out, "Canvas 500", &clock)?;
    let book = store.create_book("Lienzo", "manual", None, None, "lienzo", &clock)?;
    let mut note = store.create_note(&book, &format!("Canvas {blocks} bloques"), &clock)?;
    for index in 0..blocks {
        let mut block = Block::text(vec![
            folio_core::model::Paragraph::styled("h3", &format!("Bloque {index}")),
            folio_core::model::Paragraph::body(
                "Texto de prueba del lienzo con varios renglones para medir el flujo.",
            ),
        ]);
        if index % 10 == 0 {
            block.placement = folio_core::Placement::free(
                820.0 + (index as f64 % 3.0) * 220.0,
                (index as f64) * 12.0,
                220.0,
                90.0,
            );
        }
        note.blocks.push(block);
    }
    store.save_note(&book, &mut note, &clock)?;
    println!("{}", out.display());
    Ok(())
}
