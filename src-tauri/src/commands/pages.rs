use std::path::{Path, PathBuf};

use tauri::State;

use crate::markdown;
use crate::models::{AppError, CmdResult, IntoCmdResult, PageContent, PageNode};
use crate::paths::{
    branch_dir_for, ink_sidecar_for, normalize_virtual, resolve_page_md, split_parent_title,
    validate_title, ORDER_FILE, PAGE_FILE, PAGE_INK_FILE,
};
use crate::state::AppState;

fn vault_root(state: &State<'_, AppState>) -> Result<PathBuf, AppError> {
    state
        .vault_root()
        .ok_or(AppError::NoVault)
}

#[tauri::command]
pub async fn set_vault(
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<()> {
    state.set_vault(PathBuf::from(path)).into_cmd()
}

#[tauri::command]
pub async fn current_vault(state: State<'_, AppState>) -> CmdResult<Option<String>> {
    Ok(state.vault_root().map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn read_page_tree(
    state: State<'_, AppState>,
    virtual_path: Option<String>,
) -> CmdResult<Vec<PageNode>> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let normalized = normalize_virtual(virtual_path.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;
    list_children(&root, &normalized).into_cmd()
}

fn list_children(vault: &Path, parent_virtual: &str) -> Result<Vec<PageNode>, AppError> {
    let parent_dir = if parent_virtual.is_empty() {
        vault.to_path_buf()
    } else {
        let mut p = vault.to_path_buf();
        for seg in parent_virtual.split('/') {
            p.push(seg);
        }
        // If parent is a leaf md, look in same-named directory; otherwise use as-is.
        if !p.is_dir() {
            // No directory => no children.
            return Ok(Vec::new());
        }
        p
    };

    if !parent_dir.exists() {
        return Ok(Vec::new());
    }

    let order = read_order_file(&parent_dir);
    let mut seen = std::collections::HashSet::<String>::new();
    let mut titles: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&parent_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }
        let path = entry.path();
        if path.is_file() {
            if let Some(stem) = name.strip_suffix(".md") {
                if seen.insert(stem.to_string()) {
                    titles.push(stem.to_string());
                }
            }
        } else if path.is_dir() {
            if path.join(PAGE_FILE).exists() && seen.insert(name.clone()) {
                titles.push(name);
            }
        }
    }

    let mut ordered: Vec<String> = Vec::new();
    let mut remaining: std::collections::BTreeSet<String> = titles.into_iter().collect();
    for t in &order {
        if remaining.remove(t) {
            ordered.push(t.clone());
        }
    }
    for t in remaining.into_iter() {
        ordered.push(t);
    }

    let mut nodes = Vec::with_capacity(ordered.len());
    for title in ordered {
        let leaf = parent_dir.join(format!("{}.md", title));
        let branch = parent_dir.join(&title).join(PAGE_FILE);
        let md_path = if branch.exists() { branch.clone() } else { leaf };
        let v_path = if parent_virtual.is_empty() {
            title.clone()
        } else {
            format!("{}/{}", parent_virtual, title)
        };
        let has_children = parent_dir.join(&title).is_dir()
            && dir_has_pages(&parent_dir.join(&title));
        let has_ink = ink_sidecar_for(&md_path).exists();
        nodes.push(PageNode {
            path: v_path,
            title,
            has_children,
            has_ink,
        });
    }
    Ok(nodes)
}

fn dir_has_pages(dir: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in rd.flatten() {
        let n = entry.file_name();
        let n = n.to_string_lossy();
        if n.starts_with('.') || n.starts_with('_') {
            continue;
        }
        if n.ends_with(".md") {
            return true;
        }
        if entry.path().is_dir() && entry.path().join(PAGE_FILE).exists() {
            return true;
        }
    }
    false
}

fn read_order_file(dir: &Path) -> Vec<String> {
    let path = dir.join(ORDER_FILE);
    std::fs::read_to_string(&path)
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn write_order_file(dir: &Path, order: &[String]) -> Result<(), AppError> {
    if order.is_empty() {
        let _ = std::fs::remove_file(dir.join(ORDER_FILE));
        return Ok(());
    }
    std::fs::write(dir.join(ORDER_FILE), order.join("\n"))?;
    Ok(())
}

#[tauri::command]
pub async fn read_page(
    state: State<'_, AppState>,
    virtual_path: String,
) -> CmdResult<PageContent> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    read_page_inner(&root, &virtual_path).into_cmd()
}

fn read_page_inner(vault: &Path, virtual_path: &str) -> Result<PageContent, AppError> {
    let md = resolve_page_md(vault, virtual_path)?;
    if !md.exists() {
        return Err(AppError::NotFound(virtual_path.to_string()));
    }
    let markdown = std::fs::read_to_string(&md)?;
    let has_ink = ink_sidecar_for(&md).exists();
    Ok(PageContent { markdown, has_ink })
}

#[tauri::command]
pub async fn write_page(
    state: State<'_, AppState>,
    virtual_path: String,
    markdown: String,
) -> CmdResult<()> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let md = resolve_page_md(&root, &virtual_path).into_cmd()?;
    if let Some(parent) = md.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&md, &markdown).map_err(|e| e.to_string())?;
    // Async index update — best effort.
    if let Some(idx) = state.indexer() {
        let v = virtual_path.clone();
        let body = markdown.clone();
        tauri::async_runtime::spawn(async move {
            let _ = idx.index_page(&v, &body);
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn create_page(
    state: State<'_, AppState>,
    parent_path: String,
    title: String,
) -> CmdResult<String> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let parent_v = normalize_virtual(&parent_path).map_err(|e| e.to_string())?;
    let title = title.trim().to_string();
    validate_title(&title).map_err(|e| e.to_string())?;

    let parent_dir = if parent_v.is_empty() {
        root.clone()
    } else {
        let mut p = root.clone();
        for seg in parent_v.split('/') {
            p.push(seg);
        }
        if !p.exists() {
            // parent might be a leaf-only page; promote it to a branch dir.
            let leaf = root.join(format!("{}.md", parent_v));
            if leaf.exists() {
                std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
                let dest = p.join(PAGE_FILE);
                std::fs::rename(&leaf, &dest).map_err(|e| e.to_string())?;
                // Move ink sidecar if present.
                let leaf_ink = ink_sidecar_for(&leaf);
                if leaf_ink.exists() {
                    let _ = std::fs::rename(leaf_ink, p.join(PAGE_INK_FILE));
                }
            } else {
                std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
            }
        }
        p
    };

    let leaf = parent_dir.join(format!("{}.md", title));
    if leaf.exists() || parent_dir.join(&title).join(PAGE_FILE).exists() {
        return Err(AppError::AlreadyExists(title).to_string());
    }
    std::fs::create_dir_all(&parent_dir).map_err(|e| e.to_string())?;
    std::fs::write(&leaf, format!("# {}\n\n", title)).map_err(|e| e.to_string())?;

    // Append to .order
    let mut order = read_order_file(&parent_dir);
    if !order.iter().any(|s| s == &title) {
        order.push(title.clone());
        write_order_file(&parent_dir, &order).map_err(|e| e.to_string())?;
    }

    let v = if parent_v.is_empty() {
        title.clone()
    } else {
        format!("{}/{}", parent_v, title)
    };
    if let Some(idx) = state.indexer() {
        let _ = idx.index_page(&v, &format!("# {}\n\n", title));
    }
    Ok(v)
}

#[tauri::command]
pub async fn delete_page(
    state: State<'_, AppState>,
    virtual_path: String,
) -> CmdResult<()> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    delete_page_inner(&root, &virtual_path).into_cmd()?;
    if let Some(idx) = state.indexer() {
        let _ = idx.delete_page(&virtual_path);
    }
    Ok(())
}

fn delete_page_inner(vault: &Path, virtual_path: &str) -> Result<(), AppError> {
    let md = resolve_page_md(vault, virtual_path)?;
    if md.exists() {
        std::fs::remove_file(&md).ok();
    }
    let ink = ink_sidecar_for(&md);
    if ink.exists() {
        std::fs::remove_file(&ink).ok();
    }
    if let Some(branch) = branch_dir_for(&md) {
        if branch.exists() {
            std::fs::remove_dir_all(&branch).ok();
        }
    }
    // Remove from parent .order
    let (parent_v, title) = split_parent_title(virtual_path);
    let parent_dir = if parent_v.is_empty() {
        vault.to_path_buf()
    } else {
        let mut p = vault.to_path_buf();
        for seg in parent_v.split('/') {
            p.push(seg);
        }
        p
    };
    let mut order = read_order_file(&parent_dir);
    order.retain(|t| t != &title);
    write_order_file(&parent_dir, &order)?;
    Ok(())
}

#[tauri::command]
pub async fn rename_page(
    state: State<'_, AppState>,
    virtual_path: String,
    new_title: String,
) -> CmdResult<String> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let new_title = new_title.trim().to_string();
    validate_title(&new_title).map_err(|e| e.to_string())?;
    rename_page_inner(&root, &virtual_path, &new_title)
        .map_err(|e| e.to_string())
        .and_then(|new_v| {
            if let Some(idx) = state.indexer() {
                let _ = idx.delete_page(&virtual_path);
                if let Ok(body) = read_page_inner(&root, &new_v) {
                    let _ = idx.index_page(&new_v, &body.markdown);
                }
            }
            Ok(new_v)
        })
}

fn rename_page_inner(
    vault: &Path,
    virtual_path: &str,
    new_title: &str,
) -> Result<String, AppError> {
    let md = resolve_page_md(vault, virtual_path)?;
    if !md.exists() {
        return Err(AppError::NotFound(virtual_path.into()));
    }
    let (parent_v, old_title) = split_parent_title(virtual_path);
    let parent_dir = md
        .parent()
        .map(|p| {
            if p.file_name().and_then(|s| s.to_str()) == Some(old_title.as_str()) {
                p.parent().unwrap_or(p).to_path_buf()
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| vault.to_path_buf());

    let is_branch = md
        .file_name()
        .and_then(|s| s.to_str())
        .map(|n| n == PAGE_FILE)
        .unwrap_or(false);

    let new_md = if is_branch {
        let new_dir = parent_dir.join(new_title);
        if new_dir.exists() {
            return Err(AppError::AlreadyExists(new_title.into()));
        }
        let old_dir = md.parent().unwrap().to_path_buf();
        std::fs::rename(&old_dir, &new_dir)?;
        new_dir.join(PAGE_FILE)
    } else {
        let new_path = parent_dir.join(format!("{}.md", new_title));
        if new_path.exists() {
            return Err(AppError::AlreadyExists(new_title.into()));
        }
        std::fs::rename(&md, &new_path)?;
        let old_ink = ink_sidecar_for(&md);
        if old_ink.exists() {
            let new_ink = ink_sidecar_for(&new_path);
            let _ = std::fs::rename(old_ink, new_ink);
        }
        new_path
    };

    // Update .order
    let mut order = read_order_file(&parent_dir);
    let mut updated = false;
    for t in order.iter_mut() {
        if t == &old_title {
            *t = new_title.to_string();
            updated = true;
        }
    }
    if !updated {
        order.push(new_title.to_string());
    }
    write_order_file(&parent_dir, &order)?;

    let _ = new_md;
    let new_v = if parent_v.is_empty() {
        new_title.to_string()
    } else {
        format!("{}/{}", parent_v, new_title)
    };
    Ok(new_v)
}

#[tauri::command]
pub async fn move_page(
    state: State<'_, AppState>,
    virtual_path: String,
    new_parent: String,
) -> CmdResult<String> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let new_parent = normalize_virtual(&new_parent).map_err(|e| e.to_string())?;
    move_page_inner(&root, &virtual_path, &new_parent)
        .map_err(|e| e.to_string())
        .and_then(|new_v| {
            if let Some(idx) = state.indexer() {
                let _ = idx.delete_page(&virtual_path);
                if let Ok(body) = read_page_inner(&root, &new_v) {
                    let _ = idx.index_page(&new_v, &body.markdown);
                }
            }
            Ok(new_v)
        })
}

fn move_page_inner(
    vault: &Path,
    virtual_path: &str,
    new_parent: &str,
) -> Result<String, AppError> {
    let md = resolve_page_md(vault, virtual_path)?;
    if !md.exists() {
        return Err(AppError::NotFound(virtual_path.into()));
    }
    let (_, title) = split_parent_title(virtual_path);

    let new_parent_dir = if new_parent.is_empty() {
        vault.to_path_buf()
    } else {
        let mut p = vault.to_path_buf();
        for seg in new_parent.split('/') {
            p.push(seg);
        }
        p
    };
    std::fs::create_dir_all(&new_parent_dir)?;

    let is_branch = md
        .file_name()
        .and_then(|s| s.to_str())
        .map(|n| n == PAGE_FILE)
        .unwrap_or(false);

    if is_branch {
        let old_dir = md.parent().unwrap().to_path_buf();
        let new_dir = new_parent_dir.join(&title);
        if new_dir.exists() {
            return Err(AppError::AlreadyExists(title.clone()));
        }
        std::fs::rename(&old_dir, &new_dir)?;
    } else {
        let new_md = new_parent_dir.join(format!("{}.md", title));
        if new_md.exists() {
            return Err(AppError::AlreadyExists(title.clone()));
        }
        std::fs::rename(&md, &new_md)?;
        let old_ink = ink_sidecar_for(&md);
        if old_ink.exists() {
            let _ = std::fs::rename(old_ink, ink_sidecar_for(&new_md));
        }
    }

    // Update both .order files
    let (old_parent_v, _) = split_parent_title(virtual_path);
    let old_parent_dir = if old_parent_v.is_empty() {
        vault.to_path_buf()
    } else {
        let mut p = vault.to_path_buf();
        for seg in old_parent_v.split('/') {
            p.push(seg);
        }
        p
    };
    let mut old_order = read_order_file(&old_parent_dir);
    old_order.retain(|t| t != &title);
    write_order_file(&old_parent_dir, &old_order)?;

    let mut new_order = read_order_file(&new_parent_dir);
    if !new_order.iter().any(|t| t == &title) {
        new_order.push(title.clone());
        write_order_file(&new_parent_dir, &new_order)?;
    }

    let new_v = if new_parent.is_empty() {
        title
    } else {
        format!("{}/{}", new_parent, title)
    };
    Ok(new_v)
}

#[tauri::command]
pub async fn reorder_siblings(
    state: State<'_, AppState>,
    parent_path: String,
    order: Vec<String>,
) -> CmdResult<()> {
    let root = vault_root(&state).map_err(|e| e.to_string())?;
    let parent_v = normalize_virtual(&parent_path).map_err(|e| e.to_string())?;
    let parent_dir = if parent_v.is_empty() {
        root.clone()
    } else {
        let mut p = root.clone();
        for seg in parent_v.split('/') {
            p.push(seg);
        }
        p
    };
    write_order_file(&parent_dir, &order).into_cmd()
}

#[tauri::command]
pub async fn render_markdown(markdown: String) -> CmdResult<String> {
    Ok(markdown::render(&markdown))
}
