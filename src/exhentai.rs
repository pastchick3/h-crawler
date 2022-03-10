use regex::Regex;

use kuchiki::traits::*;
use kuchiki::{self, NodeRef};
use log::{debug, error, info};
use std::cell::RefCell;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::{self, Write};
use std::iter::zip;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lazy_static::lazy_static;

use crate::crawler::Crawler;

struct Progress {
    title: String,
    done: RefCell<usize>,
    total: usize,
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} => {}/{}", self.title, self.done.borrow(), self.total)
    }
}

impl Progress {
    fn new(title: &str, total: usize) -> Self {
        Progress {
            title: title.to_string(),
            done: RefCell::new(0),
            total,
        }
    }

    fn make_progress(&self) {
        *self.done.borrow_mut() += 1;
    }

    fn print_progress(&self) {
        print!("\r{}", self);
        io::stdout().flush().unwrap();
    }
}

pub fn crawl(crawler: Crawler, output: PathBuf, reload: usize, galleries: Vec<String>) {
    for gallery in galleries {
        let parts: Vec<_> = gallery.split('/').collect();
        if parts.len() != 3 {
            panic!("Invalid gallery `{}`.", gallery);
        }
        let url = format!("https://exhentai.org/g/{}/{}/", parts[0], parts[1]);
        let range = match parts[2] {
            "" => None,
            range => {
                let range: Vec<_> = range.split('-').collect();
                if range.len() != 2 {
                    panic!("Invalid range `{}`.", gallery);
                }
                let start = range[0].parse().unwrap();
                let end = range[1].parse().unwrap();
                Some((start, end))
            }
        };
        crawl_gallery(&crawler, &output, reload, url, range);
    }
}

fn crawl_gallery(
    crawler: &Crawler,
    output: &PathBuf,
    reload: usize,
    url: String,
    range: Option<(usize, usize)>,
) {
    // Crawl the home page and extract some basic information.
    let mut page = match crawler.get_text(&url, Vec::new()) {
        Ok(page) => page,
        Err(err) => {
            error!("Fail to request the index page 1 for `{}`: {}", url, err);
            return;
        }
    };
    let document = kuchiki::parse_html().one(page);
    let title = extract_title(&document).unwrap();
    let image_count = extract_image_count(&document).unwrap();

    // Determine a proper range.
    let (start, end) = match range {
        Some((start, end)) => (start - 1, end),
        None => (0, image_count),
    };

    let progress = Progress::new(&title, end - start);
    progress.print_progress();

    let start_page = start / 20;
    let start = start % 20;
    let end_page = end / 20;
    let end = (end_page - start_page) * 20 + end % 20;

    info!(
        "Crawl `{}` from {}:{} to {}:{} at `{}`.",
        title, start_page, start, end_page, end, url
    );

    // Crawl index pages and extract links to image pages.
    let mut image_page_urls = Vec::new();
    let tasks = (start_page..=end_page)
        .into_iter()
        .map(|p| (url.as_str(), vec![("p".to_string(), p.to_string())]))
        .collect();
    let results = crawler.batch_text(tasks);
    for (p, result) in results.into_iter().enumerate() {
        let page = match result {
            Ok(page) => page,
            Err(err) => {
                error!(
                    "Fail to request the index page {} for `{url}`: {err}",
                    p + 1,
                );
                return;
            }
        };
        let document = kuchiki::parse_html().one(page);
        let urls = extract_image_page_urls(&document).unwrap();
        image_page_urls.extend(urls);
    }

    // Create the gallery directory.
    let title = sanitize_filename::sanitize(title);
    let mut folder_path = output.clone();
    folder_path.push(&title);
    fs::create_dir(output)
        .map_err(|err| error!("Fail to create the gallery directory for `{title}`: {err}"))
        .unwrap();

    // Crawl image pages and images.
    let mut image_pages: Vec<_> = image_page_urls[start..end]
        .iter()
        .map(|url| (url, Vec::new(), Vec::new()))
        .collect();
    for _ in 0..=reload {
        let uncrawler_pages: Vec<_> = image_pages
            .iter_mut()
            .filter(|(_, _, image)| image.is_empty())
            .collect();
        let page_tasks = uncrawler_pages
            .iter()
            .map(|(u, q, i)| (u.as_str(), q.to_vec()))
            .collect();
        let page_results = crawler.batch_text(page_tasks);

        let uncrawler_images: Vec<_> = zip(page_results, uncrawler_pages)
            .filter_map(|(result, (_, queries, image))| {
                if let Ok(img) = result {
                    let document = kuchiki::parse_html().one(img);
                    let (image_url, new_query) = extract_image_urls(&document).unwrap();
                    queries.push(new_query);
                    Some((image_url, image))
                } else {
                    None
                }
            })
            .collect();

        let image_tasks = uncrawler_images
            .iter()
            .map(|(u, i)| (u.as_str(), Vec::new()))
            .collect();
        let image_results = crawler.batch(image_tasks);
        for (result, (_, image)) in zip(image_results, uncrawler_images) {
            if let Ok(img) = result {
                *image = img;
                // self.progress.make_progress();
                // self.progress.print_progress();
            }
        }
    }

    // Write to file.
    for (i, (url, _, image)) in image_pages.iter().enumerate() {
        if !image.is_empty() {
            lazy_static! {
                static ref FILE_EXTENSION_REGEX: Regex = Regex::new(r"\.[^\.]+?$").unwrap();
            }
            let caps = FILE_EXTENSION_REGEX
                .captures(url)
                .ok_or(format!("Invalid file extension: {url}"))
                .unwrap();
            let file_name = format!("{:0>4}{}", i + 1, &caps[0]);
            let mut file_path = folder_path.clone();
            file_path.push(file_name);
            let mut file = fs::File::create(file_path)
                .map_err(|err| format!("Fail to create the image file: {err}"))
                .unwrap();
            file.write_all(&image).expect("Fail to write the image.");
        }
    }

    // if !self.verbose {
    //     println!();
    // }

    // Print failed images.
    let failed_images: Vec<_> = image_pages
        .iter()
        .enumerate()
        .filter_map(
            |(i, (_, _, image))| {
                if image.is_empty() {
                    Some(i + 1)
                } else {
                    None
                }
            },
        )
        .collect();
    if !failed_images.is_empty() {
        println!("Fail to crawl the following images:");
        let mut buffer = String::new();
        for i in failed_images {
            buffer.push_str(&format!("{i}, "));
        }
        buffer.pop();
        buffer.pop();
        println!("{buffer}");
    }
}

fn extract_title(document: &NodeRef) -> Result<String, String> {
    match document.select_first("#gj") {
        Ok(title) => Ok(title.text_contents()),
        Err(_) => match document.select_first("#gn") {
            Ok(title) => Ok(title.text_contents()),
            Err(_) => Err(String::from("Fail to locate the gallery title")),
        },
    }
}

fn extract_image_count(document: &NodeRef) -> Result<usize, ()> {
    let length_field = document
        .select_first("#gdd tr:nth-child(6) td:nth-child(2)")?
        .text_contents();
    lazy_static! {
        static ref IMAGE_COUNT_REGEX: Regex = Regex::new(r"(\d+) .+").unwrap();
    }
    let caps = IMAGE_COUNT_REGEX.captures(&length_field).ok_or(())?;
    caps[1].parse().map_err(|_| ())
}

fn extract_image_page_urls(document: &NodeRef) -> Result<Vec<String>, ()> {
    let a_tags = document.select("#gdt a")?;
    a_tags
        .map(|a| {
            Ok(a.as_node().clone()
                .into_element_ref()
                .ok_or(())?
                .attributes
                .borrow()
                .get("href")
                .ok_or(())?
                .to_string())
        })
        .collect()
}

fn extract_image_urls(document: &NodeRef) -> Result<(String, (String, String)), ()> {
    // Extract the image url.
    let image_url = document
        .select_first("#img")?
        .as_node().clone()
        .into_element_ref()
        .ok_or(())?
        .attributes
        .borrow()
        .get("src")
        .ok_or(())?
        .to_string();

    // Extract the reload parameters.
    let reload_fn = document
        .select_first("#loadfail")?
        .as_node().clone()
        .into_element_ref()
        .ok_or(())?
        .attributes
        .borrow()
        .get("onclick")
        .ok_or(())?
        .to_string();
    lazy_static! {
        static ref RELOAD_REGEX: Regex = Regex::new(r"return (.+?)\('(.+?)'\)").unwrap();
    }
    let caps = RELOAD_REGEX.captures(&reload_fn).ok_or(())?;

    Ok((image_url, (caps[1].to_string(), caps[2].to_string())))
}
