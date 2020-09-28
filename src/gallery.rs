use log::{debug, error, info, warn};
use std::fmt;

pub struct Gallery {
    artist: String,
    title: String,
    url: String,
    range: Option<(usize, usize)>,
}

impl Gallery {
    pub fn new(artist: &str, title: &str, url: &str, range: Option<(usize, usize)>) -> Self {
        Gallery {
            artist: String::from(artist),
            title: String::from(title),
            url: String::from(url),
            range
        }
    }
}

// [にろ] えろえるふの湯2 @ /g/1720798/fae55905ba/ pp. 1-26 => Complete
impl fmt::Display for Gallery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((start, end)) = self.range {
            write!(f, "[{}] {} @ {} pp. {}-{}", self.artist, self.title, self.url, start, end)
        } else {
            write!(f, "[{}] {} @ {}", self.artist, self.title, self.url)
        }
    }
}