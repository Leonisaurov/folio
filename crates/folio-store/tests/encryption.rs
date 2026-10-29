use std::fs;
use std::path::Path;

use folio_core::{Block, FixedClock};
use folio_store::Store;
use folio_store::assets;
use folio_store::crypto;

fn walk(root: &Path) -> Vec<std::path::PathBuf> {
    folio_store::atomic::walk_files(root).unwrap()
}

#[test]
fn encrypted_vault_roundtrip_and_no_plaintext_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let (store, phrase) =
        Store::init_encrypted(dir.path(), "Secreta", &clock, "contraseña").unwrap();
    assert!(store.is_encrypted());
    assert!(store.is_unlocked());
    assert_eq!(phrase.split_whitespace().count(), 24);

    let book = store
        .create_book("Proyecto secreto", "manual", None, None, "proyecto", &clock)
        .unwrap();
    let mut note = store.create_note(&book, "Título secreto", &clock).unwrap();
    note.blocks
        .push(Block::text_plain("contenido confidencial del jardín"));
    store.save_note(&book, &mut note, &clock).unwrap();
    let asset = assets::import_bytes(
        &store.vault,
        b"bytes del adjunto secreto",
        "txt",
        store.crypto(),
    )
    .unwrap();
    let display = store.asset_display_path(&asset).unwrap().unwrap();
    assert_eq!(fs::read(&display).unwrap(), b"bytes del adjunto secreto");

    for path in walk(dir.path()) {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        assert!(!name.starts_with(".tmp-"), "temporal sin limpiar: {name}");
        if path.ends_with("vault.toml")
            || path.ends_with("keyring.json")
            || path.to_string_lossy().contains("/.folio/cache/plain/")
        {
            continue;
        }
        let bytes = fs::read(&path).unwrap();
        assert!(
            crypto::is_encrypted(&bytes),
            "archivo sin cifrar: {}",
            path.display()
        );
        let haystack = String::from_utf8_lossy(&bytes);
        assert!(!haystack.contains("contenido confidencial"));
        assert!(!haystack.contains("Título secreto"));
        assert!(!haystack.contains("bytes del adjunto secreto"));
    }
    assert!(
        !fs::read(dir.path().join("vault.toml"))
            .unwrap()
            .windows(b"contenido confidencial".len())
            .any(|window| window == b"contenido confidencial")
    );

    drop(store);
    let locked = Store::open(dir.path()).unwrap();
    assert!(locked.is_encrypted());
    assert!(!locked.is_unlocked());
    assert!(locked.list_books().is_err());

    assert!(Store::open_with_password(dir.path(), "mala").is_err());
    let opened = Store::open_with_password(dir.path(), "contraseña").unwrap();
    let book = opened.book(&book.meta.id).unwrap();
    let restored = opened.load_note(&book, &note.id).unwrap();
    assert_eq!(restored.title, "Título secreto");
    assert!(restored.plain_text().contains("contenido confidencial"));
    assert_eq!(
        assets::read_bytes(&opened.vault, &asset, opened.crypto()).unwrap(),
        b"bytes del adjunto secreto"
    );

    let recovered = Store::open_with_recovery(dir.path(), &phrase).unwrap();
    let note2 = recovered
        .load_note(&recovered.book(&book.meta.id).unwrap(), &note.id)
        .unwrap();
    assert_eq!(note2.title, "Título secreto");
}

#[test]
fn lock_blocks_io_and_versions_trash_are_encrypted() {
    let dir = tempfile::tempdir().unwrap();
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let (mut store, _) = Store::init_encrypted(dir.path(), "Secreta", &clock, "clave").unwrap();
    let book = store
        .create_book("Libro", "manual", None, None, "libro", &clock)
        .unwrap();
    let mut note = store.create_note(&book, "Nota", &clock).unwrap();
    note.blocks.push(Block::text_plain("versión secreta"));
    store.save_note(&book, &mut note, &clock).unwrap();
    let snapshot =
        folio_store::versions::maybe_snapshot(&store.vault, &note, &clock, true, store.crypto())
            .unwrap()
            .unwrap();
    let raw = fs::read(&snapshot).unwrap();
    assert!(crypto::is_encrypted(&raw));
    assert!(!String::from_utf8_lossy(&raw).contains("versión secreta"));
    let restored = folio_store::versions::read(&snapshot, store.crypto()).unwrap();
    assert!(restored.plain_text().contains("versión secreta"));

    let trashed = store.delete_note(&book, &note.id, &clock).unwrap();
    let meta = fs::read(trashed.join("meta.json")).unwrap();
    assert!(crypto::is_encrypted(&meta));
    assert!(!String::from_utf8_lossy(&meta).contains("Nota"));
    let entries = folio_store::trash::list(&store.vault, store.crypto()).unwrap();
    assert_eq!(entries.len(), 1);

    store.lock();
    assert!(!store.is_unlocked());
    assert!(
        !store.vault.folio_dir().join("cache/plain").exists(),
        "la caché descifrada debe borrarse al bloquear"
    );
    assert!(store.load_note(&book, &note.id).is_err());
    store.unlock("clave").unwrap();
    assert!(store.is_unlocked());
    assert!(store.load_note(&book, &note.id).is_err());
}

#[test]
fn encrypt_and_decrypt_existing_vault() {
    let dir = tempfile::tempdir().unwrap();
    let clock = FixedClock::at("2026-10-29T09:00:00Z");
    let mut store = Store::init(dir.path(), "Plana", &clock).unwrap();
    let book = store
        .create_book("Libro", "manual", None, None, "libro", &clock)
        .unwrap();
    let mut note = store.create_note(&book, "Nota plana", &clock).unwrap();
    note.blocks
        .push(Block::text_plain("texto que debe cifrarse"));
    store.save_note(&book, &mut note, &clock).unwrap();
    let phrase = store.encrypt_in_place("clave").unwrap();
    assert!(store.is_encrypted());
    assert_eq!(phrase.split_whitespace().count(), 24);
    let opened = Store::open_with_password(dir.path(), "clave").unwrap();
    assert_eq!(
        opened.load_note(&book, &note.id).unwrap().title,
        "Nota plana"
    );
    let mut plain = Store::open_with_password(dir.path(), "clave").unwrap();
    plain.decrypt_in_place("clave").unwrap();
    assert!(!plain.is_encrypted());
    assert!(!crypto::keyring_path(dir.path()).exists());
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(
        reopened.load_note(&book, &note.id).unwrap().title,
        "Nota plana"
    );
}
