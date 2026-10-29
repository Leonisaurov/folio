use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use folio_core::FolioError;
use notify::{RecursiveMode, Watcher};

/// Vigila la bóveda con debounce y notifica cambios agrupados.
/// El receptor se cierra cuando se suelta el handle devuelto.
pub struct VaultWatcher {
    _watcher: notify::RecommendedWatcher,
    pub events: Receiver<()>,
}

pub fn watch(root: &Path, debounce: Duration) -> Result<VaultWatcher, FolioError> {
    let (raw_tx, raw_rx) = channel::<()>();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        if let Ok(event) = result
            && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
        {
            let _ = raw_tx.send(());
        }
    })
    .map_err(|error| FolioError::Io(std::io::Error::other(error)))?;
    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|error| FolioError::Io(std::io::Error::other(error)))?;

    let (events_tx, events_rx) = channel::<()>();
    std::thread::spawn(move || {
        while raw_rx.recv().is_ok() {
            let deadline = Instant::now() + debounce;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match raw_rx.recv_timeout(remaining) {
                    Ok(()) => continue,
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            if events_tx.send(()).is_err() {
                return;
            }
        }
    });

    Ok(VaultWatcher {
        _watcher: watcher,
        events: events_rx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;
    use folio_core::SystemClock;

    #[test]
    fn detects_external_change_with_debounce() {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::init(dir.path(), "v", &SystemClock).unwrap();
        let watcher = watch(&vault.root, Duration::from_millis(100)).unwrap();
        std::fs::write(vault.root.join("books").join("externo.txt"), b"x").unwrap();
        let received = watcher.events.recv_timeout(Duration::from_secs(5));
        assert!(received.is_ok(), "el watcher debe detectar el cambio");
    }
}
