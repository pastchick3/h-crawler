use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, Result as SqlResult};

/// Generic result type used by the repository.
type RepositoryResult<T> = Result<T, String>;

#[derive(Debug)]
struct Metadata {
    gid: u64,
    token: String,
    archiver_key: String,
    title: String,
    title_jpn: String,
    category: String,
    thumb: String,
    uploader: String,
    posted: String,
    filecount: String,
    filesize: u64,
    expunged: bool,
    rating: String,
    torrentcount: String,
    tags: String, // jsonify of tags in querier::Metadata
    date: String, // ISO 8601 YYYY-MM-DD
    path: Option<String> // new entry
}

/// repository
///     - index.db
///     - gallery
///         - pics
pub struct Repository {
    path: PathBuf,
    conn: Connection,
}

impl Repository {
    pub fn new(path: &Path) -> RepositoryResult<Self> {
        if !path.is_dir() {
            return Err(String::from("Invalid directory."))
        }
        let mut db_path = path.to_path_buf();
        db_path.push("index.db");
        let conn = Connection::open(db_path.as_path()).map_err(|err| format!("{:?}", err))?;
        Ok(Repository {
            path: path.to_path_buf(),
            conn,
        })
    }

    pub fn insert(&self, metadata: Metadata) -> RepositoryResult<()> {
        todo!()
    }

    pub fn delete(&self, gid: u64) -> RepositoryResult<()> {
        todo!()
    }

    pub fn update(&self, metadata: Metadata) -> RepositoryResult<()> {
        todo!()
    }

    // Macro
    // pub fn select(&self, gid: u64) -> RepositoryResult<()> {
    //     todo!()
    // }
}
