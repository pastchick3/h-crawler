use crate::crawler::Crawler;
use kuchiki::traits::*;
use kuchiki::{self, NodeRef};
use lazy_static::lazy_static;
use regex::Regex;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

struct Image {
    page_url: String,
    reload_values: Vec<String>,
    image_url: String,
    result: Result<Vec<u8>, String>,
}

pub fn crawl_galleries(crawler: &Crawler, output: PathBuf, reload: usize, galleries: Vec<String>) {
    for gallery in galleries {
        // Process the gallery id and the range.
        let parts: Vec<_> = gallery.split('/').collect();
        if parts.len() != 3 {
            println!("Invalid Gallery {gallery}");
            continue;
        }
        let id = format!("{}/{}/", parts[0], parts[1]);
        let range = parts[2];
        let url = format!("https://exhentai.org/g/{id}");

        // Crawl gallery's home page.
        let home_result = crawler
            .get_text("", vec![(&url, Vec::new())])
            .pop()
            .unwrap();
        let page = match home_result {
            Ok(page) => page,
            Err(err) => {
                println!("Fail to crawl the home page for Gallery {id}: {err}");
                continue;
            }
        };

        // Extract the gallery title and the image count.
        let document = kuchiki::parse_html().one(page);
        let title = extract_title(&document);
        let count = extract_count(&document);

        // Create the gallery directory.
        let mut directory_path = output.clone();
        let directory = sanitize_filename::sanitize(&title);
        directory_path.push(&directory);
        fs::create_dir(&directory_path).unwrap();

        // Determine a proper range.
        let (start, end) = if range.is_empty() {
            (1, count)
        } else {
            let range: Vec<_> = range.split('-').collect();
            if range.len() != 2 {
                println!("Invalid range for Gallery {gallery}");
                continue;
            }
            let start = range[0].parse().unwrap();
            let end = range[1].parse().unwrap();
            (start, end)
        };
        let start_page = (start - 1) / 20;
        let start = start - start_page * 20 - 1;
        let end_page = (end - 1) / 20 + 1;
        let end = end - start_page * 20;

        // Crawl index pages.
        let indexes: Vec<_> = (start_page..end_page).map(|i| i.to_string()).collect();
        let index_requests = indexes
            .iter()
            .map(|i| (url.as_str(), vec![("p", i.as_str())]))
            .collect();
        let index_results = crawler.get_text("", index_requests);

        // Extract links to image pages.
        let mut image_page_urls = Vec::new();
        for (index, index_result) in (start_page..end_page).zip(index_results) {
            let pg = index + 1;
            let page = match index_result {
                Ok(page) => page,
                Err(err) => {
                    println!("Fail to crawl index page {pg} for Gallery {id}: {err}");
                    continue;
                }
            };
            let document = kuchiki::parse_html().one(page);
            image_page_urls.extend(extract_image_page_urls(&document));
        }

        // Initialize image tasks.
        let mut images: Vec<_> = image_page_urls
            .drain(start..end)
            .map(|page_url| Image {
                page_url,
                reload_values: Vec::new(),
                image_url: String::new(),
                result: Err(String::new()),
            })
            .collect();

        for r in 0..=reload {
            // Crawl image pages.
            let uncrawled_images: Vec<_> = images
                .iter_mut()
                .filter(|image| image.result.is_err())
                .collect();
            if uncrawled_images.is_empty() {
                break;
            }
            let image_page_requests = uncrawled_images
                .iter()
                .map(|image| {
                    (
                        image.page_url.as_str(),
                        image
                            .reload_values
                            .iter()
                            .map(|v| ("nl", v.as_str()))
                            .collect(),
                    )
                })
                .collect();
            let image_page_results =
                crawler.get_text(&format!("{title} (page, reload={r})"), image_page_requests);

            // Crawl images.
            let uncrawled_images: Vec<_> = uncrawled_images
                .into_iter()
                .zip(image_page_results)
                .filter_map(|(image, result)| match result {
                    Ok(page) => {
                        let document = kuchiki::parse_html().one(page);
                        image.image_url = extract_image_url(&document);
                        image.reload_values.push(extract_reload_value(&document));
                        Some(image)
                    }
                    Err(err) => {
                        image.result = Err(err);
                        None
                    }
                })
                .collect();
            let image_requests = uncrawled_images
                .iter()
                .map(|image| (image.image_url.as_str(), Vec::new()))
                .collect();
            let image_results =
                crawler.get_byte(&format!("{title} (image, reload={r})"), image_requests);
            for (image, result) in uncrawled_images.into_iter().zip(image_results) {
                image.result = result;
            }
        }

        // Write images to local files.
        for (i, image) in images.iter().enumerate() {
            let pg = i + 1;
            match &image.result {
                Ok(img) => {
                    let ext = {
                        lazy_static! {
                            static ref EXT_REGEX: Regex = Regex::new(r"\.[^\.]+$").unwrap();
                        }
                        let caps = EXT_REGEX.captures(&image.image_url).unwrap();
                        caps[0].to_string()
                    };
                    let mut path = directory_path.clone();
                    path.push(format!("{pg:0>4}{ext}"));
                    let mut file = File::create(path).unwrap();
                    file.write_all(img).unwrap();
                }
                Err(err) => println!("Fail to crawl page {pg} for Gallery {id}: {err}"),
            }
        }
    }
}

fn extract_title(document: &NodeRef) -> String {
    let title = document.select_first("#gj").unwrap().text_contents();
    if title.is_empty() {
        document.select_first("#gn").unwrap().text_contents()
    } else {
        title
    }
}

fn extract_count(document: &NodeRef) -> usize {
    let count = document.select_first(".gpc").unwrap().text_contents();
    lazy_static! {
        static ref COUNT_REGEX: Regex = Regex::new(r"([,\d]+)\s+images$").unwrap();
    }
    let caps = COUNT_REGEX.captures(&count).unwrap();
    caps[1].replace(',', "").parse().unwrap()
}

fn extract_image_page_urls(document: &NodeRef) -> Vec<String> {
    document
        .select("#gdt a")
        .unwrap()
        .map(|a| a.attributes.borrow().get("href").unwrap().to_string())
        .collect()
}

fn extract_image_url(document: &NodeRef) -> String {
    document
        .select_first("#img")
        .unwrap()
        .attributes
        .borrow()
        .get("src")
        .unwrap()
        .to_string()
}

fn extract_reload_value(document: &NodeRef) -> String {
    let loadfail = document
        .select_first("#loadfail")
        .unwrap()
        .attributes
        .borrow()
        .get("onclick")
        .unwrap()
        .to_string();
    lazy_static! {
        static ref RELOAD_REGEX: Regex = Regex::new(r"'(.+?)'").unwrap();
    }
    let caps = RELOAD_REGEX.captures(&loadfail).unwrap();
    caps[1].to_string()
}
