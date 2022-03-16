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
    image: Vec<u8>,
    error: String,
}

pub fn crawl_galleries(crawler: &Crawler, output: PathBuf, reload: usize, galleries: Vec<String>) {
    for gallery in galleries {
        let parts: Vec<_> = gallery.split('/').collect();
        if parts.len() != 3 {
            println!("Invalid Gallery {gallery}");
            continue;
        }
        let url = format!("https://exhentai.org/g/{}/{}/", parts[0], parts[1]);
        let range = parts[2];

        // Crawl the home page and extract some basic information.
        let page_result = crawler
            .get_text("", vec![(&url, Vec::new())])
            .pop()
            .unwrap();
        let page = match page_result {
            Ok(page) => page,
            Err(err) => {
                println!("Fail to crawl the home page for Gallery {gallery}: {err}");
                continue;
            }
        };
        let document = kuchiki::parse_html().one(page);
        let title = extract_title(&document);
        let image_count = extract_image_count(&document);

        // Create the gallery directory.
        let mut directory_path = output.clone();
        directory_path.push(sanitize_filename::sanitize(&title));
        fs::create_dir(&directory_path).unwrap();

        // Determine a proper range.
        let (start, end) = if range.is_empty() {
            (1, image_count)
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

        // Crawl index pages and extract links to image pages.
        let index_pgs: Vec<_> = (start_page..end_page).map(|i| i.to_string()).collect();
        let index_requests = index_pgs
            .iter()
            .map(|pg| (url.as_str(), vec![("p", pg.as_str())]))
            .collect();
        let index_results = crawler.get_text("", index_requests);
        let mut image_page_urls = Vec::new();
        for (index_pg, index_result) in index_pgs.iter().zip(index_results) {
            let page = match index_result {
                Ok(page) => page,
                Err(err) => {
                    println!("Fail to crawl index page {index_pg} for Gallery {gallery}: {err}");
                    continue;
                }
            };
            let document = kuchiki::parse_html().one(page);
            image_page_urls.extend(extract_image_page_urls(&document));
        }

        // Crawl image pages and images.
        let mut images: Vec<_> = image_page_urls
            .drain(start..end)
            .map(|page_url| Image {
                page_url,
                reload_values: Vec::new(),
                image_url: String::new(),
                image: Vec::new(),
                error: String::new(),
            })
            .collect();
        for r in 0..=reload {
            let uncrawled_images: Vec<_> = images
                .iter_mut()
                .filter(|img| img.image.is_empty())
                .collect();

            if uncrawled_images.is_empty() {
                break;
            }

            let image_page_requests = uncrawled_images
                .iter()
                .map(|img| {
                    (
                        img.page_url.as_str(),
                        img.reload_values
                            .iter()
                            .map(|v| ("nl", v.as_str()))
                            .collect(),
                    )
                })
                .collect();
            let image_page_results =
                crawler.get_text(&format!("{title} (page, reload={r})"), image_page_requests);

            let uncrawled_images: Vec<_> = image_page_results
                .into_iter()
                .zip(uncrawled_images)
                .filter_map(|(result, image)| match result {
                    Ok(page) => {
                        println!("ok {}", page.len());
                        let document = kuchiki::parse_html().one(page);
                        image.image_url = extract_image_url(&document);
                        image.reload_values.push(extract_reload_value(&document));
                        Some(image)
                    }
                    Err(err) => {
                        println!("err {err}");
                        image.error = err;
                        None
                    }
                })
                .collect();

            let image_requests = uncrawled_images
                .iter()
                .map(|img| (img.image_url.as_str(), Vec::new()))
                .collect();
            let image_results =
                crawler.get_byte(&format!("{title} (image, reload={r})"), image_requests);
            image_results.into_iter().zip(uncrawled_images).for_each(
                |(result, image)| match result {
                    Ok(img) => image.image = img,
                    Err(err) => image.error = err,
                },
            );
        }

        // Write to file.
        for (i, image) in images.iter().enumerate() {
            let pg = i + 1;
            if image.image.is_empty() {
                println!(
                    "Fail to crawl page {pg} for Gallery {gallery}: {}",
                    &image.error
                );
            } else {
                let image_ext = {
                    lazy_static! {
                        static ref IMAGE_EXT_REGEX: Regex = Regex::new(r"\.[^\.]+$").unwrap();
                    }
                    let caps = IMAGE_EXT_REGEX.captures(&image.image_url).unwrap();
                    caps[0].to_string()
                };

                let mut image_path = directory_path.clone();
                image_path.push(format!("{pg:0>4}{image_ext}"));
                let mut image_file = File::create(image_path).unwrap();
                image_file.write_all(&image.image).unwrap();
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

fn extract_image_count(document: &NodeRef) -> usize {
    let image_count = document.select_first(".gpc").unwrap().text_contents();
    lazy_static! {
        static ref IMAGE_COUNT_REGEX: Regex = Regex::new(r"(\d+)\s+images$").unwrap();
    }
    let caps = IMAGE_COUNT_REGEX.captures(&image_count).unwrap();
    caps[1].parse().unwrap()
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
