use crate::crawler::Crawler;
use std::path::PathBuf;
use kuchiki::traits::*;
use serde_json::{Value};
use lazy_static::lazy_static;
use regex::Regex;
use std::fs;
use log::{error};
use std::io::Write;

pub fn crawl_users(crawler: Crawler, output: PathBuf, ids: Vec<String>) {

}

pub fn crawl_artworks(crawler: Crawler, output: PathBuf, ids: Vec<String>) {
    for id in ids {
    let page = crawler.get_text(&format!("https://www.pixiv.net/artworks/{id}"), Vec::new()).unwrap();
    let document = kuchiki::parse_html().one(page);
    let json_str = document
        .select_first("#meta-preload-data").unwrap()
        .as_node().clone()
        .into_element_ref()
        .ok_or(()).unwrap()
        .attributes
        .borrow()
        .get("content")
        .ok_or(()).unwrap()
        .to_string();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        let obj = &json["illust"][&id];
        let artist = obj["userName"].as_str().unwrap();
        let raw_date = obj["createDate"].as_str().unwrap();
        lazy_static! {
            static ref DATE_REGEX: Regex = Regex::new(r"^[0-9]{2}([0-9]{2})-([0-9]{2})-([0-9]{2})").unwrap();
        }
        let caps = DATE_REGEX
                .captures(raw_date)
                .unwrap();
        let date = format!("{}{}{}",&caps[1],&caps[2],&caps[3]);
        let title = obj["title"].as_str().unwrap();
        let page_count = obj["pageCount"].as_u64().unwrap();
        let raw_url = obj["urls"]["original"].as_str().unwrap();
        lazy_static! {
            static ref URL_RGEX: Regex = Regex::new(r"(.+)[0-9]+(\.[a-z]+)$").unwrap();
        }
        let caps = URL_RGEX
                .captures(raw_url)
                .unwrap();
        let image_base = &caps[1];
        let image_ext = &caps[2];


        // Create the gallery directory.
        let title = format!("[{artist}] [{date}] {title} ({id})");
        let title = sanitize_filename::sanitize(title);
        let mut folder_path = output.clone();
        folder_path.push(&title);
        fs::create_dir(&folder_path)
            .map_err(|err| error!("Fail to create the gallery directory for `{title}`: {err}"))
            .unwrap();


        // crawl image
        for i in 0..page_count {
            let url = format!("{image_base}{i}{image_ext}");
            let image = crawler.get(&url, Vec::new()).unwrap();
            let mut image_path = folder_path.clone();
            image_path.push(format!("{id}_p{i}.{image_ext}"));
            let mut file = fs::File::create(image_path)
                .map_err(|err| format!("Fail to create the image file: {err}"))
                .unwrap();
            file.write_all(&image).expect("Fail to write the image.");
        }
    }

}


// path :: String
// path = "./[コーラ] [200611] 雨の町 (82255171)"

// main :: IO ()
// main = do
//   crawlArtwork "82255171"
//   files <- listDirectory path
//   p0Size <- getFileSize (path ++ "/82255171_p0.jpg")
//   p1Size <- getFileSize (path ++ "/82255171_p1.jpg")
//   p2Size <- getFileSize (path ++ "/82255171_p2.jpg")
//   case (length files, p0Size, p1Size, p2Size) of
//     (3, 2282431, 2108130, 1400864) -> removePathForcibly path >> exitSuccess
//     _ -> exitFailure
