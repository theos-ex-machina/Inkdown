use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, STRING, TEXT};
use tantivy::snippet::SnippetGenerator;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use walkdir::WalkDir;

use crate::models::{AppError, SearchHit};
use crate::paths::{fs_to_virtual, INDEX_DIR, PAGE_FILE};

const WRITER_HEAP: usize = 64 * 1024 * 1024;

pub struct VaultIndex {
    pub vault_root: PathBuf,
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    path_field: Field,
    title_field: Field,
    body_field: Field,
}

fn build_schema() -> (Schema, Field, Field, Field) {
    let mut builder = Schema::builder();
    let path_field = builder.add_text_field("path", STRING | STORED);
    let title_field = builder.add_text_field("title", TEXT | STORED);
    let body_field = builder.add_text_field("body", TEXT | STORED);
    let schema = builder.build();
    (schema, path_field, title_field, body_field)
}

impl VaultIndex {
    pub fn open_or_create(vault_root: &Path) -> Result<Self, AppError> {
        let dir = vault_root.join(INDEX_DIR).join("index");
        std::fs::create_dir_all(&dir)?;
        let (schema, path_field, title_field, body_field) = build_schema();
        let mmap = MmapDirectory::open(&dir).map_err(|e| AppError::Tantivy(e.to_string()))?;
        let exists = Index::exists(&mmap).map_err(|e| AppError::Tantivy(e.to_string()))?;
        let index = if exists {
            Index::open_in_dir(&dir)?
        } else {
            Index::create_in_dir(&dir, schema)?
        };
        let writer = index.writer(WRITER_HEAP)?;
        let reader: IndexReader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        Ok(Self {
            vault_root: vault_root.to_path_buf(),
            index,
            reader,
            writer: Mutex::new(writer),
            path_field,
            title_field,
            body_field,
        })
    }

    /// Walk the vault and (re)index every .md file. Cheap when the index is
    /// already up to date because tantivy de-dupes by `path`.
    pub fn full_rebuild(&self) -> Result<(), AppError> {
        let mut writer = self.writer.lock().map_err(|_| AppError::Other("indexer mutex".into()))?;
        writer
            .delete_all_documents()
            .map_err(|e| AppError::Tantivy(e.to_string()))?;
        for entry in WalkDir::new(&self.vault_root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(e.depth() > 0 && name.starts_with('.') && name != PAGE_FILE)
            })
        {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !(name == PAGE_FILE || name.ends_with(".md")) {
                continue;
            }
            let Some(vpath) = fs_to_virtual(&self.vault_root, path) else {
                continue;
            };
            let body = std::fs::read_to_string(path).unwrap_or_default();
            let title = title_for(&vpath);
            writer.add_document(doc!(
                self.path_field => vpath,
                self.title_field => title,
                self.body_field => body,
            ))?;
        }
        writer.commit()?;
        Ok(())
    }

    pub fn index_page(&self, vpath: &str, body: &str) -> Result<(), AppError> {
        let mut writer = self.writer.lock().map_err(|_| AppError::Other("indexer mutex".into()))?;
        let term = tantivy::Term::from_field_text(self.path_field, vpath);
        writer.delete_term(term);
        let title = title_for(vpath);
        writer.add_document(doc!(
            self.path_field => vpath.to_string(),
            self.title_field => title,
            self.body_field => body.to_string(),
        ))?;
        writer.commit()?;
        Ok(())
    }

    pub fn delete_page(&self, vpath: &str) -> Result<(), AppError> {
        let mut writer = self.writer.lock().map_err(|_| AppError::Other("indexer mutex".into()))?;
        let term = tantivy::Term::from_field_text(self.path_field, vpath);
        writer.delete_term(term);
        writer.commit()?;
        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, AppError> {
        let searcher = self.reader.searcher();
        let parser = QueryParser::for_index(&self.index, vec![self.title_field, self.body_field]);
        let parsed = parser.parse_query(query)?;
        let collector = TopDocs::with_limit(limit).order_by_score();
        let top_docs = searcher.search(&*parsed, &collector)?;

        let mut hits = Vec::with_capacity(top_docs.len());
        let snippet_gen = SnippetGenerator::create(&searcher, &*parsed, self.body_field)?;

        for (score, addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(addr)?;
            let path = doc
                .get_first(self.path_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title_raw = doc
                .get_first(self.title_field)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let snippet = snippet_gen.snippet_from_doc(&doc).to_html();
            let title = if title_raw.is_empty() {
                title_for(&path)
            } else {
                title_raw
            };
            hits.push(SearchHit {
                path,
                title,
                snippet,
                score,
            });
        }
        Ok(hits)
    }
}

fn title_for(vpath: &str) -> String {
    vpath
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("Untitled")
        .to_string()
}
