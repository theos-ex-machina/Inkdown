use std::path::{Path, PathBuf};

use crate::models::AppError;

pub const PAGE_FILE: &str = ".page.md";
pub const PAGE_INK_FILE: &str = ".page.ink.json";
pub const ORDER_FILE: &str = ".order";
pub const INDEX_DIR: &str = ".inkdown";
pub const ASSETS_DIR: &str = "_assets";

const FORBIDDEN_TITLE_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// Validate a single path segment (title) for filesystem safety.
pub fn validate_title(title: &str) -> Result<(), AppError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidPath("empty title".into()));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(AppError::InvalidPath("reserved title".into()));
    }
    for c in trimmed.chars() {
        if FORBIDDEN_TITLE_CHARS.contains(&c) || (c as u32) < 0x20 {
            return Err(AppError::ForbiddenChar(c));
        }
    }
    Ok(())
}

/// Sanitize a freeform title for use as a filename.
pub fn sanitize_title(title: &str) -> String {
    let trimmed = title.trim();
    trimmed
        .chars()
        .map(|c| {
            if FORBIDDEN_TITLE_CHARS.contains(&c) || (c as u32) < 0x20 {
                '-'
            } else {
                c
            }
        })
        .collect()
}

/// Normalize a virtual page path (the kind the frontend exchanges):
/// forward slashes, no leading/trailing slash, no `.` or `..` segments,
/// no `.md` extension.
pub fn normalize_virtual(path: &str) -> Result<String, AppError> {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split(['/', '\\']) {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        if seg == "." {
            continue;
        }
        if seg == ".." {
            return Err(AppError::InvalidPath(format!("traversal in '{}'", path)));
        }
        for c in seg.chars() {
            if FORBIDDEN_TITLE_CHARS.contains(&c) || (c as u32) < 0x20 {
                return Err(AppError::ForbiddenChar(c));
            }
        }
        out.push(seg);
    }
    Ok(out.join("/"))
}

/// Split a normalized virtual path into (parent_dir_virtual, title).
pub fn split_parent_title(virtual_path: &str) -> (String, String) {
    match virtual_path.rsplit_once('/') {
        Some((parent, title)) => (parent.to_string(), title.to_string()),
        None => (String::new(), virtual_path.to_string()),
    }
}

/// Resolve a normalized virtual page path into an absolute filesystem path
/// to its markdown file inside `vault_root`. Both branch (directory + .page.md)
/// and leaf (Foo.md) layouts are checked; if neither exists, we return the
/// leaf path so callers can decide how to handle the missing case.
pub fn resolve_page_md(vault_root: &Path, virtual_path: &str) -> Result<PathBuf, AppError> {
    let normalized = normalize_virtual(virtual_path)?;
    if normalized.is_empty() {
        return Ok(vault_root.join(PAGE_FILE));
    }
    let (parent, title) = split_parent_title(&normalized);
    let parent_dir = if parent.is_empty() {
        vault_root.to_path_buf()
    } else {
        let mut p = vault_root.to_path_buf();
        for seg in parent.split('/') {
            p.push(seg);
        }
        p
    };
    let branch_md = parent_dir.join(&title).join(PAGE_FILE);
    let leaf_md = parent_dir.join(format!("{}.md", title));
    if branch_md.exists() {
        Ok(branch_md)
    } else if leaf_md.exists() {
        Ok(leaf_md)
    } else {
        Ok(leaf_md)
    }
}

/// Resolve the sidecar `.ink.json` path that pairs with a given `.md` path.
pub fn ink_sidecar_for(md_path: &Path) -> PathBuf {
    let file_name = md_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if file_name == PAGE_FILE {
        md_path
            .parent()
            .map(|p| p.join(PAGE_INK_FILE))
            .unwrap_or_else(|| PathBuf::from(PAGE_INK_FILE))
    } else {
        let stem = md_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        md_path
            .with_file_name(format!("{}.ink.json", stem))
    }
}

/// For a given `.md` path, return the directory that holds its potential
/// subpages (branch directory). For `Foo.md` that's `Foo/`; for `Foo/.page.md`
/// that's `Foo/`.
pub fn branch_dir_for(md_path: &Path) -> Option<PathBuf> {
    let file_name = md_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    if file_name == PAGE_FILE {
        md_path.parent().map(|p| p.to_path_buf())
    } else {
        let stem = md_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        md_path
            .parent()
            .map(|p| p.join(stem))
    }
}

/// Convert an absolute path inside the vault to a virtual page path.
/// Returns `None` if the path isn't a markdown page.
pub fn fs_to_virtual(vault_root: &Path, abs: &Path) -> Option<String> {
    let rel = abs.strip_prefix(vault_root).ok()?;
    let file_name = rel.file_name()?.to_str()?;
    let parent = rel.parent().unwrap_or(Path::new(""));
    let parent_str = parent.to_string_lossy().replace('\\', "/");
    if file_name == PAGE_FILE {
        if parent_str.is_empty() {
            Some(String::new())
        } else {
            Some(parent_str)
        }
    } else if let Some(stem) = file_name.strip_suffix(".md") {
        if parent_str.is_empty() {
            Some(stem.to_string())
        } else {
            Some(format!("{}/{}", parent_str, stem))
        }
    } else {
        None
    }
}
