use log::{debug, error, info, warn};
use rusqlite::{Connection, Result, params};
use std::fmt;
use std::path::PathBuf;

pub struct Gallery {
    artist: String,
    title: String,
    url: String,
    start: Option<u16>,
    end: Option<u16>,
}

impl fmt::Display for Gallery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            write!(f, "[{}] {} @ {} pp. {}-{}", self.artist, self.title, self.url, start, end)
        } else {
            write!(f, "[{}] {} @ {}", self.artist, self.title, self.url)
        }
    }
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let path = PathBuf::from("./galleries.db");
        let conn = if path.is_file() {
            Connection::open(path)?
        } else {
            let conn = Connection::open(path)?;
            conn.execute(
                "CREATE TABLE Galleries (
                    artist TEXT NOT NULL,
                    title TEXT NOT NULL,
                    url TEXT NOT NULL,
                    start INTEGER,
                    end INTEGER,
                    PRIMARY KEY (artist, title)
                );",
                params![],
            )?;
            conn
        };
        Ok(Database {
            conn,
        })
    }

    pub fn add(
        &self,
        artist: &str,
        title: &str,
        url: &str,
        start: Option<u16>,
        end: Option<u16>,
    ) -> Result<usize> {
        self.conn.execute(
            "INSERT INTO Galleries VALUES (?1, ?2, ?3, ?4, ?5);",
            params![artist, title, url, start, end],
        )
    }

    pub fn remove(&self, artist: Option<&str>, title: Option<&str>) -> Result<usize> {
        let (sql, params) = self.assemble_sql("DELETE FROM Galleries", artist, title);
        self.conn.execute(
            &sql,
            params,
        )
    }

    pub fn find(&self, artist: Option<&str>, title: Option<&str>) -> Result<Vec<Gallery>> {
        let (sql, params) = self.assemble_sql("SELECT artist, title, url, start, end FROM Galleries", artist, title);
        let mut stmt = self.conn.prepare(&sql)?;
        let iter = stmt.query_map(params, |row| {
            Ok(Gallery {
                artist: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                start: row.get(3)?,
                end: row.get(4)?,
            })
        })?;

        let mut galleries = Vec::new();
        for gallery in iter {
            galleries.push(gallery?);
        }
        Ok(galleries)
    }

    fn assemble_sql<'a>(&self, sql: &str, artist: Option<&'a str>, title: Option<&'a str>) -> (String, Vec<&'a str>) {
        match (artist, title) {
            (Some(artist), Some(title)) => {
                (
                    String::from(sql) + " where artist = ?1 AND title = ?2",
                    vec![artist, title],
                )
            }
            (None, Some(title)) => {
                (
                    String::from(sql) + " where title = ?1",
                    vec![title],
                )
            }
            (Some(artist), None) => {
                (
                    String::from(sql) + " where artist = ?1",
                    vec![artist],
                )
            }
            (None, None) => {
                (
                    String::from(sql),
                    vec![],
                )
            }
        }
    }
}
