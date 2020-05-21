use std::path::Path;


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
    tags: String, // different from querier::Metadata
    path: Option<String> // new entry
}

pub struct Repository {

}

impl Repository {
    pub fn new(path: &Path) -> Self {
        Repository {

        }
    }
}