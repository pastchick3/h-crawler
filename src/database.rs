use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use rusqlite::{Connection, Result as SQLResult, params};

use crate::crawler::Crawler;
use crate::gallery::Gallery;

static RUNNING: bool = true;

pub struct Database {
    task_tx: Sender<Gallery>,
    result_rx: Receiver<Gallery>,
    conn: Connection,
}

impl Database {
    pub fn new(username: String, password: String, resource: PathBuf) -> SQLResult<Self> {
        let path = resource.join("galleries.db");
        let (task_tx, task_rx) = channel();
        let (result_tx, result_rx) = channel();

        thread::spawn(move|| {
            let crawler = Crawler::new(username, password, resource, task_rx, result_tx);
        });

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
                    PRIMARY KEY (artist, title),
                )",
                params![],
            )?;
            conn
        };
        Ok(Database {
            task_tx,
            result_rx,
            conn,
        })
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

    pub fn find(&self, artist: Option<&str>, title: Option<&str>) -> Vec<Gallery> {
        println!("find {:?} {:?}", artist, title);
        Vec::new()
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        println!("Dropping database!");
    }
}
