use std::path::{Path, PathBuf};

use tauri::State;

use crate::models::{AppError, CmdResult, IntoCmdResult};
use crate::paths::{resolve_page_md, sanitize_title, ASSETS_DIR};
use crate::state::AppState;

fn vault_root(state: &State<'_, AppState>) -> Result<PathBuf, AppError> {
    state.vault_root().ok_or(AppError::NoVault)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetKind {
    Image,
    Pdf,
    Other,
}

fn classify(ext: &str) -> AssetKind {
    match ext.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => AssetKind::Image,
        "pdf" => AssetKind::Pdf,
        _ => AssetKind::Other,
    }
}

fn hash_file(path: &Path) -> Result<String, AppError> {
    let mut hasher = blake3::Hasher::new();
    let bytes = std::fs::read(path)?;
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

fn unique_dest(dir: &Path, base: &str, ext: &str, hash: &str) -> PathBuf {
    let short = &hash[..hash.len().min(8)];
    let candidate = if ext.is_empty() {
        format!("{}-{}", base, short)
    } else {
        format!("{}-{}.{}", base, short, ext)
    };
    dir.join(candidate)
}

#[tauri::command]
pub async fn import_asset(
    state: State<'_, AppState>,
    note_path: String,
    source_path: String,
) -> CmdResult<String> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let md = resolve_page_md(&root, &note_path).into_cmd()?;
    let note_dir = md
        .parent()
        .ok_or_else(|| AppError::InvalidPath("no parent".into()))
        .into_cmd()?
        .to_path_buf();
    let assets_dir = note_dir.join(ASSETS_DIR);
    std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("missing source file: {}", source_path));
    }

    let file_name = source
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "bad file name".to_string())?;
    let ext = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    let safe_stem = sanitize_title(stem);

    let hash = hash_file(&source).map_err(|e| e.to_string())?;
    let dest = unique_dest(&assets_dir, &safe_stem, &ext, &hash);

    if !dest.exists() {
        std::fs::copy(&source, &dest).map_err(|e| e.to_string())?;
    }

    let rel = format!("./_assets/{}", dest.file_name().unwrap().to_string_lossy());
    let kind = classify(&ext);
    let alt = safe_stem.clone();
    let markdown_ref = match kind {
        AssetKind::Image => format!("![{}]({})", alt, rel),
        AssetKind::Pdf => format!("```pdf\n{}\n```", rel),
        AssetKind::Other => format!("[{}]({})", alt, rel),
    };
    Ok(markdown_ref)
}
