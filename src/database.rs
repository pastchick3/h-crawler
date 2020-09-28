use log::{debug, error, info, warn};
use std::fmt;
use std::path::Path;

use crate::crawler::Crawler;

pub struct Record {}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "()")
    }
}

pub struct Database {
    crawler: Crawler,
}

impl Database {
    pub fn new(username: &str, password: &str, resource: &Path, debug: bool) -> Self {
        println!("start {} {} {:?} {}", username, password, resource, debug);
        Database {
            crawler: Crawler::new(username, password),
        }
    }

    pub fn status(&self) -> Vec<String> {
        println!("status");
        Vec::new()
    }

    pub fn add(
        &self,
        artist: &str,
        title: &str,
        url: &str,
        range: Option<(usize, usize)>,
    ) -> Result<(), String> {
        println!("add {} {} {} {:?}", artist, title, url, range);
        Ok(())
    }

    pub fn remove(&self, artist: Option<&str>, title: Option<&str>) -> Result<(), String> {
        println!("remove {:?} {:?}", artist, title);
        Ok(())
    }

    pub fn find(&self, artist: Option<&str>, title: Option<&str>) -> Vec<Record> {
        println!("find {:?} {:?}", artist, title);
        Vec::new()
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        println!("Dropping database!");
    }
}

// [にろ] えろえるふの湯2 @ /g/1720798/fae55905ba/ pp. 1-26 => Complete

// use std::path::{Path, PathBuf};

// use rusqlite::{params, Connection, Result as SqlResult};

// /// Generic result type used by the repository.
// type RepositoryResult<T> = Result<T, String>;

// #[derive(Debug)]
// struct Metadata {
//     gid: u32,
//     token: String,
//     archiver_key: String,
//     title: String,
//     title_jpn: String,
//     category: String,
//     thumb: String,
//     uploader: String,
//     posted: String,
//     filecount: String,
//     filesize: u32,
//     expunged: bool,
//     rating: String,
//     torrentcount: String,
//     tags: String,         // jsonify of tags in querier::Metadata
//     date: String,         // new entry, TEXT as ISO8601 strings ("YYYY-MM-DD HH:MM:SS.SSS")
//     path: Option<String>, // new entry
// }

// /// repository
// ///     - index.db
// ///     - gallery
// ///         - pics
// pub struct Repository {
//     path: PathBuf,
//     conn: Connection,
// }

// impl Repository {
//     pub fn new(path: &Path) -> RepositoryResult<Self> {
//         if !path.is_dir() {
//             return Err(String::from("Invalid directory."));
//         }
//         let mut db_path = path.to_path_buf();
//         db_path.push("index.db");
//         let conn = if db_path.is_file() {
//             Connection::open(db_path.as_path()).map_err(|err| format!("{:?}", err))?
//         } else {
//             let conn = Connection::open(db_path.as_path()).map_err(|err| format!("{:?}", err))?;
//             conn.execute(
//                 "CREATE TABLE index (
//                     gid INTEGER PRIMARY KEY,
//                     token TEXT NOT NULL,
//                     archiver_key TEXT NOT NULL,
//                     title TEXT NOT NULL,
//                     title_jpn TEXT NOT NULL,
//                     category TEXT NOT NULL,
//                     thumb TEXT NOT NULL,
//                     uploader TEXT NOT NULL,
//                     posted TEXT NOT NULL,
//                     filecount TEXT NOT NULL,
//                     filesize INTEGER NOT NULL,
//                     expunged: BOOLEAN NOT NULL,
//                     rating TEXT NOT NULL,
//                     torrentcount TEXT NOT NULL,
//                     tags TEXT NOT NULL,
//                     date TEXT NOT NULL,
//                     path TEXT,
//                 )",
//                 params![],
//             )
//             .map_err(|err| format!("{:?}", err))?;
//             conn
//         };
//         Ok(Repository {
//             path: path.to_path_buf(),
//             conn,
//         })
//     }

//     pub fn insert(&self, metadata: Metadata) -> RepositoryResult<()> {
//         self.conn.execute(
//             "INSERT INTO index VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
//             params![
//                 metadata.gid,
//                 metadata.token,
//                 metadata.archiver_key,
//                 metadata.title,
//                 metadata.title_jpn,
//                 metadata.category,
//                 metadata.thumb,
//                 metadata.uploader,
//                 metadata.posted,
//                 metadata.filecount,
//                 metadata.filesize,
//                 metadata.expunged,
//                 metadata.rating,
//                 metadata.torrentcount,
//                 metadata.tags,
//                 metadata.date,
//                 metadata.path,
//             ],
//         ).map_err(|err| format!("{:?}", err))?;
//         Ok(())
//     }

//     pub fn delete(&self, gid: u64) -> RepositoryResult<()> {
//         todo!()
//     }

//     pub fn update(&self, metadata: Metadata) -> RepositoryResult<()> {
//         todo!()
//     }

//     // Macro
//     // pub fn select(&self, gid: u64) -> RepositoryResult<()> {
//     //     todo!()
//     // }
// }
