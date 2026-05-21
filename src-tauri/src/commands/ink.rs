use std::path::PathBuf;

use tauri::State;

use crate::models::{AppError, CmdResult, InkDocument, IntoCmdResult};
use crate::paths::{ink_sidecar_for, resolve_page_md};
use crate::state::AppState;

fn vault_root(state: &State<'_, AppState>) -> Result<PathBuf, AppError> {
    state.vault_root().ok_or(AppError::NoVault)
}

#[tauri::command]
pub async fn read_ink(
    state: State<'_, AppState>,
    virtual_path: String,
) -> CmdResult<Option<InkDocument>> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let md = resolve_page_md(&root, &virtual_path).into_cmd()?;
    let ink = ink_sidecar_for(&md);
    if !ink.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&ink).map_err(|e| e.to_string())?;
    let doc: InkDocument = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if doc.version != 1 {
        return Err(format!("unsupported ink version {}", doc.version));
    }
    Ok(Some(doc))
}

#[tauri::command]
pub async fn write_ink(
    state: State<'_, AppState>,
    virtual_path: String,
    ink: InkDocument,
) -> CmdResult<()> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let md = resolve_page_md(&root, &virtual_path).into_cmd()?;
    if let Some(parent) = md.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let path = ink_sidecar_for(&md);
    if ink.strokes.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let body = serde_json::to_string(&ink).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    Ok(())
}
