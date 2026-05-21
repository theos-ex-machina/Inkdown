use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNode {
    /// Virtual path relative to vault root, using forward slashes, no extension.
    /// Examples: "Getting Started", "Projects/Alpha".
    pub path: String,
    pub title: String,
    pub has_children: bool,
    pub has_ink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageContent {
    pub markdown: String,
    pub has_ink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkDocument {
    pub version: u32,
    pub strokes: Vec<Stroke>,
}

impl Default for InkDocument {
    fn default() -> Self {
        Self {
            version: 1,
            strokes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stroke {
    /// Each point is `[x, y, pressure, tiltX, tiltY, timestamp]`.
    pub points: Vec<[f64; 6]>,
    pub color: String,
    pub width: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("walkdir: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("tantivy: {0}")]
    Tantivy(String),
    #[error("tantivy query: {0}")]
    TantivyQuery(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("forbidden character in title: '{0}'")]
    ForbiddenChar(char),
    #[error("vault not opened")]
    NoVault,
    #[error("{0}")]
    Other(String),
}

impl From<tantivy::TantivyError> for AppError {
    fn from(e: tantivy::TantivyError) -> Self {
        AppError::Tantivy(e.to_string())
    }
}

impl From<tantivy::query::QueryParserError> for AppError {
    fn from(e: tantivy::query::QueryParserError) -> Self {
        AppError::TantivyQuery(e.to_string())
    }
}

/// Convert internal errors into strings at the command boundary so the
/// frontend can render them as toasts.
pub type CmdResult<T> = Result<T, String>;

pub trait IntoCmdResult<T> {
    fn into_cmd(self) -> CmdResult<T>;
}

impl<T> IntoCmdResult<T> for Result<T, AppError> {
    fn into_cmd(self) -> CmdResult<T> {
        self.map_err(|e| e.to_string())
    }
}
