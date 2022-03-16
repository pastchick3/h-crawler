use crate::crawler::Crawler;
use kuchiki::traits::*;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub fn crawl_users(crawler: &Crawler, output: PathBuf, users: Vec<String>) {
    for user in users {
        let parts: Vec<_> = user.split('/').collect();
        let (id, range) = match parts[..] {
            [id] => (id, None),
            [id, range] => (id, Some(range)),
            _ => {
                println!("Invalid User {user}");
                continue;
            }
        };

        let page_result = crawler
            .get_text(
                "",
                vec![(&format!("https://www.pixiv.net/users/{id}"), Vec::new())],
            )
            .pop()
            .unwrap();
        let user = match page_result {
            Ok(page) => {
                let document = kuchiki::parse_html().one(page);
                let json_str = document
                    .select_first("#meta-preload-data")
                    .unwrap()
                    .attributes
                    .borrow()
                    .get("content")
                    .unwrap()
                    .to_string();
                let json: Value = serde_json::from_str(&json_str).unwrap();
                json["user"][&id]["name"].as_str().unwrap().to_string()
            }
            Err(err) => {
                println!("Fail to crawl the home page for User {id}: {err}");
                continue;
            }
        };

        let mut directory_path = output.clone();
        let directory = sanitize_filename::sanitize(format!("[{user}]"));
        directory_path.push(directory);
        fs::create_dir(&directory_path).unwrap();

        let illusts_result = crawler
            .get_json(
                "",
                vec![(
                    &format!("https://www.pixiv.net/ajax/user/{id}/profile/all"),
                    Vec::new(),
                )],
            )
            .pop()
            .unwrap();

        let mut illusts: Vec<_> = match illusts_result {
            Ok(json) => json["body"]["illusts"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .rev()
                .collect(),
            Err(err) => {
                println!("Fail to crawl the illust index for User {id}: {err}");
                continue;
            }
        };

        let (start, end) = if let Some(range) = range {
            let parts: Vec<_> = range.split('-').collect();
            if parts.len() != 2 {
                println!("Invalid range for User {id}");
                continue;
            }
            let start = parts[0].parse().unwrap();
            let end = parts[1].parse().unwrap();
            (start, end)
        } else {
            (1, illusts.len())
        };

        let total = end - start + 1;
        println!("{user} - {total} illusts");

        crawl_illusts(
            crawler,
            directory_path,
            illusts.drain(start - 1..end).collect(),
        );
    }
}

pub fn crawl_illusts(crawler: &Crawler, output: PathBuf, illusts: Vec<String>) {
    let page_urls: Vec<_> = illusts
        .iter()
        .map(|id| format!("https://www.pixiv.net/artworks/{id}"))
        .collect();
    let page_requests = page_urls
        .iter()
        .map(|url| (url.as_str(), Vec::new()))
        .collect();
    let page_results = crawler.get_text("", page_requests);

    let index_urls: Vec<_> = illusts
        .iter()
        .map(|id| format!("https://www.pixiv.net/ajax/illust/{id}/pages"))
        .collect();
    let index_requests = index_urls
        .iter()
        .map(|url| (url.as_str(), Vec::new()))
        .collect();
    let index_results = crawler.get_json("", index_requests);

    let illusts = illusts
        .iter()
        .zip(page_results)
        .zip(index_results)
        .filter_map(|((id, page), index)| match (page, index) {
            (Ok(page), Ok(index)) => Some((id, page, index)),
            (Ok(_), Err(err)) => {
                println!("Fail to crawl the image index for Illust {id}: {err}");
                None
            }
            (Err(err), Ok(_)) => {
                println!("Fail to crawl the main page for Illust {id}: {err}");
                None
            }
            (Err(page_err), Err(index_err)) => {
                println!("Fail to crawl the main page for Illust {id}: {page_err}");
                println!("Fail to crawl the image index for Illust {id}: {index_err}");
                None
            }
        });

    for (id, page, index) in illusts {
        let document = kuchiki::parse_html().one(page);
        let json_str = document
            .select_first("#meta-preload-data")
            .unwrap()
            .attributes
            .borrow()
            .get("content")
            .unwrap()
            .to_string();
        let json: Value = serde_json::from_str(&json_str).unwrap();
        let illust = &json["illust"][&id];
        let user = illust["userName"].as_str().unwrap();
        let date = {
            let date = illust["createDate"].as_str().unwrap();
            lazy_static! {
                static ref DATE_REGEX: Regex =
                    Regex::new(r"([0-9]{2})-([0-9]{2})-([0-9]{2})").unwrap();
            }
            let caps = DATE_REGEX.captures(date).unwrap();
            format!("{}{}{}", &caps[1], &caps[2], &caps[3])
        };
        let title = illust["title"].as_str().unwrap();

        // Create the gallery directory.
        let title = sanitize_filename::sanitize(format!("[{user}] [{date}] {title} ({id})"));
        let mut directory_path = output.clone();
        directory_path.push(&title);
        fs::create_dir(&directory_path).unwrap();

        // crawl image
        let image_urls: Vec<_> = index["body"]
            .as_array()
            .unwrap()
            .iter()
            .map(|image| image["urls"]["original"].as_str().unwrap())
            .collect();
        let image_requests = image_urls.iter().map(|url| (*url, Vec::new())).collect();
        let image_results = crawler.get_byte(&title, image_requests);
        let images = image_results
            .iter()
            .enumerate()
            .filter_map(|(pg, result)| match result {
                Ok(image) => Some((pg, image)),
                Err(err) => {
                    println!("Fail to crawl page {pg} for Illust {id}: {err}");
                    None
                }
            });
        for ((pg, image), image_url) in images.zip(image_urls) {
            let image_ext = {
                lazy_static! {
                    static ref IMAGE_EXT_REGEX: Regex = Regex::new(r"\.[^\.]+$").unwrap();
                }
                let caps = IMAGE_EXT_REGEX.captures(image_url).unwrap();
                caps[0].to_string()
            };

            let mut image_path = directory_path.clone();
            image_path.push(format!("{id}_p{pg}{image_ext}"));
            let mut image_file = File::create(image_path).unwrap();
            image_file.write_all(image).unwrap();
        }
    }
}
