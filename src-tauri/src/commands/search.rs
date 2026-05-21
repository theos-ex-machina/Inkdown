use tauri::State;

use crate::models::{CmdResult, SearchHit};
use crate::state::AppState;

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> CmdResult<Vec<SearchHit>> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let Some(idx) = state.indexer() else {
        return Ok(Vec::new());
    };
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    idx.search(query.trim(), limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rebuild_index(state: State<'_, AppState>) -> CmdResult<()> {
    let Some(idx) = state.indexer() else {
        return Err("vault not opened".into());
    };
    idx.full_rebuild().map_err(|e| e.to_string())
}
