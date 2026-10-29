use std::path::Path;
use std::process::Command;

fn folio() -> Command {
    Command::new(env!("CARGO_BIN_EXE_folio"))
}

fn run(args: &[&str]) -> String {
    let output = folio().args(args).output().expect("ejecutar folio");
    assert!(
        output.status.success(),
        "folio {:?} falló:\nstdout: {}\nstderr: {}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn init_vault(path: &Path) -> String {
    run(&["vault", "init", path.to_str().unwrap(), "--name", "CLI"])
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn cli_end_to_end_vault_book_note_search_export() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    let init = init_vault(&vault);
    assert!(init.contains("Bóveda creada"));

    let book = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "book",
        "create",
        "--title",
        "Proyectos",
    ]);
    let book_id = book.trim().lines().last().unwrap().to_string();
    assert!(!book_id.is_empty());

    let note = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "note",
        "create",
        "--book",
        &book_id,
        "--title",
        "Jardín",
        "--text",
        "El jardín tiene tomates y albahaca",
    ]);
    let note_id = note.trim().to_string();
    assert!(!note_id.is_empty());

    run(&["--vault", vault.to_str().unwrap(), "index", "rebuild"]);
    let search = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "search",
        "jardin",
        "--json",
    ]);
    assert!(search.contains(&note_id), "búsqueda sin acentos: {search}");

    let out = dir.path().join("nota");
    let exported = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "export",
        "--format",
        "html",
        "--note",
        &note_id,
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(exported.contains("nota.html"));
    let html = std::fs::read_to_string(dir.path().join("nota.html")).unwrap();
    assert!(html.contains("jardín"));
    assert!(!html.contains("<script"));

    let doctor = run(&["--vault", vault.to_str().unwrap(), "doctor", "--json"]);
    assert!(doctor.contains("\"notes\": 1"));

    let config = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "config",
        "get",
        "daily.title_format",
    ]);
    assert_eq!(config.trim(), "%d/%m/%Y");
}

#[test]
fn cli_config_writes_vault_toml_and_validates() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    init_vault(&vault);
    run(&[
        "--vault",
        vault.to_str().unwrap(),
        "config",
        "set",
        "daily.title_format",
        "%Y/%m/%d",
    ]);
    let text = std::fs::read_to_string(vault.join("vault.toml")).unwrap();
    assert!(text.contains("title_format = \"%Y/%m/%d\""));
    let value = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "config",
        "get",
        "daily.title_format",
    ]);
    assert_eq!(value.trim(), "%Y/%m/%d");
    run(&["--vault", vault.to_str().unwrap(), "config", "validate"]);
}

#[test]
fn cli_daily_book_and_backfill() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    init_vault(&vault);
    let book = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "book",
        "create",
        "--title",
        "2026-10-29",
        "--daily",
        "--date",
        "2026-10-29",
    ]);
    assert!(!book.trim().is_empty());
    let backfill = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "backfill",
        "--from",
        "2026-10-20",
        "--to",
        "2026-10-22",
    ]);
    assert_eq!(backfill.lines().count(), 3);
    let books = run(&["--vault", vault.to_str().unwrap(), "book", "list", "--json"]);
    assert!(books.contains("2026-10-20"));
    assert!(books.contains("2026-10-29"));
}

#[test]
fn cli_encrypted_vault_requires_password_and_leaks_no_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    let password = "contraseña-larga";
    run(&[
        "vault",
        "init",
        vault.to_str().unwrap(),
        "--encrypt",
        "--password",
        password,
    ]);
    let book = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "--password",
        password,
        "book",
        "create",
        "--title",
        "Proyecto secreto",
    ]);
    let book_id = book.trim();
    let note = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "--password",
        password,
        "note",
        "create",
        "--book",
        book_id,
        "--title",
        "Nota secreta",
        "--text",
        "contenido confidencial único",
    ]);
    let note_id = note.trim();
    let search = run(&[
        "--vault",
        vault.to_str().unwrap(),
        "--password",
        password,
        "search",
        "confidencial",
        "--json",
    ]);
    assert!(search.contains(note_id));

    let wrong = folio()
        .args([
            "--vault",
            vault.to_str().unwrap(),
            "--password",
            "incorrecta",
            "note",
            "list",
            "--book",
            book_id,
        ])
        .output()
        .unwrap();
    assert!(!wrong.status.success());

    // Ningún archivo de datos contiene el texto en claro. El índice en
    // bóvedas cifradas es solo en memoria.
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), out);
            } else {
                out.push(entry.path());
            }
        }
    }
    let mut files = Vec::new();
    walk(&vault, &mut files);
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy();
        if name == "vault.toml" || name == "keyring.json" {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("contenido confidencial"),
            "{}",
            path.display()
        );
        assert!(!text.contains("Nota secreta"), "{}", path.display());
        assert!(!text.contains("Proyecto secreto"), "{}", path.display());
    }
    assert!(!vault.join(".folio/index.sqlite").exists());
}
