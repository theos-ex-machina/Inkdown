use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::indexer::VaultIndex;
use crate::models::AppError;

#[derive(Default)]
pub struct AppState {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    vault: Option<PathBuf>,
    index: Option<Arc<VaultIndex>>,
}

impl AppState {
    pub fn vault_root(&self) -> Option<PathBuf> {
        self.inner.read().ok().and_then(|i| i.vault.clone())
    }

    pub fn indexer(&self) -> Option<Arc<VaultIndex>> {
        self.inner.read().ok().and_then(|i| i.index.clone())
    }

    /// Open (or reopen) the vault at `path`. Rebuilds the search index in the
    /// background so the UI stays responsive.
    pub fn set_vault(&self, path: PathBuf) -> Result<(), AppError> {
        std::fs::create_dir_all(&path)?;
        let index = VaultIndex::open_or_create(&path)?;
        let arc = Arc::new(index);
        {
            let mut guard = self
                .inner
                .write()
                .map_err(|_| AppError::Other("state poisoned".into()))?;
            guard.vault = Some(path);
            guard.index = Some(arc.clone());
        }
        let arc_bg = arc.clone();
        tauri::async_runtime::spawn(async move {
            let _ = arc_bg.full_rebuild();
        });
        Ok(())
    }
}
