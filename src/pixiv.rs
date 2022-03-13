use crate::crawler::Crawler;
use kuchiki::traits::*;
use lazy_static::lazy_static;
use log::error;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::iter;
use std::path::PathBuf;

pub fn crawl_users(crawler: Crawler, output: PathBuf, ids: Vec<String>) {
    for id in ids {
        // Create the artist directory.
        let json = crawler
            .get_json(
                "",
                vec![(format!("https://www.pixiv.net/users/{id}"), Vec::new())],
            )
            .pop()
            .unwrap()
            .unwrap();
        let artist = json["user"][&id]["name"].as_str().unwrap().to_string();

        let folder = format!("[{artist}]");
        let folder = sanitize_filename::sanitize(folder);
        let mut folder_path = output.clone();
        folder_path.push(&folder);
        fs::create_dir(&folder_path)
            .map_err(|err| error!("Fail to create the gallery directory for `{artist}`: {err}"))
            .unwrap();

        // Crawl all artworks
        let page = crawler
            .get_text(
                "",
                vec![(
                    format!("https://www.pixiv.net/ajax/user/{id}/profile/all"),
                    Vec::new(),
                )],
            )
            .pop()
            .unwrap()
            .unwrap();
        let json: Value = serde_json::from_str(&page).unwrap();
        let illusts: Vec<_> = json["body"]["illusts"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        crawl_artworks(&crawler, folder_path, artist, illusts);
    }
}

pub fn crawl_artworks(crawler: &Crawler, output: PathBuf, artist: String, ids: Vec<String>) {
    println!("{artist}");
    let requests = ids
        .iter()
        .map(|id| (format!("https://www.pixiv.net/artworks/{id}"), Vec::new()))
        .collect();
    let results = crawler.get_text("    Artwork Index", requests);
    let responses = iter::zip(ids, results).filter_map(|(id, result)| match result {
        Ok(response) => Some((id, response)),
        Err(err) => {
            println!("    Fail to crawl artwork {id}");
            error!("Fail to crawl artwork {id}: {err}");
            None
        }
    });
    for (id, response) in responses {
        let document = kuchiki::parse_html().one(response);
        let json_str = document
            .select_first("#meta-preload-data")
            .unwrap()
            .attributes
            .borrow()
            .get("content")
            .ok_or(())
            .unwrap()
            .to_string();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        let obj = &json["illust"][&id];
        let artist = obj["userName"].as_str().unwrap();
        let raw_date = obj["createDate"].as_str().unwrap();
        lazy_static! {
            static ref DATE_REGEX: Regex =
                Regex::new(r"^[0-9]{2}([0-9]{2})-([0-9]{2})-([0-9]{2})").unwrap();
        }
        let caps = DATE_REGEX.captures(raw_date).unwrap();
        let date = format!("{}{}{}", &caps[1], &caps[2], &caps[3]);
        let title = obj["title"].as_str().unwrap();
        let page_count = obj["pageCount"].as_u64().unwrap();
        let raw_url = obj["urls"]["original"].as_str().unwrap();
        lazy_static! {
            static ref URL_RGEX: Regex = Regex::new(r"(.+)[0-9]+(\.[a-z]+)$").unwrap();
        }
        let caps = URL_RGEX.captures(raw_url).unwrap();
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
        let requests = (0..page_count)
            .map(|i| (format!("{image_base}{i}{image_ext}"), Vec::new()))
            .collect();
        let results = crawler.get_byte(&format!("    {title}"), requests);
        let responses =
            iter::zip(0..page_count, results).filter_map(|(cnt, result)| match result {
                Ok(response) => Some((cnt, response)),
                Err(err) => {
                    println!("    Fail to crawl image {cnt}");
                    error!("Fail to crawl image {cnt}: {err}");
                    None
                }
            });
        for (i, image) in responses {
            let mut image_path = folder_path.clone();
            image_path.push(format!("{id}_p{i}.{image_ext}"));
            let mut file = fs::File::create(image_path)
                .map_err(|err| format!("Fail to create the image file: {err}"))
                .unwrap();
            file.write_all(&image).expect("Fail to write the image.");
        }
    }
}
