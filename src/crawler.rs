use crate::Credential;
use futures::future;
use regex::Regex;
use reqwest::header;
use reqwest::{Client, Error, Response};
use scraper::{Html, Selector};
use std::cell::RefCell;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::Semaphore;

const PATH: &str = ".";
const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);
const TIMEOUT: u64 = 60;
const RETRY: usize = 1;
const RELOAD: usize = 1;
const CONCURRENCY: usize = 5;

lazy_static! {
    static ref JAPANESE_TITLE_SELECTOR: Selector = Selector::parse("#gj").unwrap();
    static ref DEFAULT_TITLE_SELECTOR: Selector = Selector::parse("#gn").unwrap();
    static ref PAGE_NUM_SELECTOR: Selector =
        Selector::parse("#asm + div td:nth-last-child(2) > a").unwrap();
    static ref IMAGE_PAGE_SELECTOR: Selector = Selector::parse("#gdt a").unwrap();
    static ref FILE_NAME_EXTENSION_REGEX: Regex = Regex::new(r"\.[^\.]+?$").unwrap();
    static ref IMAGE_SELECTOR: Selector = Selector::parse("#img").unwrap();
    static ref RELOAD_SELECTOR: Selector = Selector::parse("#loadfail").unwrap();
    static ref RELOAD_REGEX: Regex = Regex::new(r"return (.+?)\('(.+?)'\)").unwrap();
}

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
    fn new() -> Self {
        Progress {
            title: String::from("_"),
            done: RefCell::new(0),
            total: 0,
        }
    }

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn set_total(&mut self, total: usize) {
        self.total = total;
    }

    fn make_progress(&self) {
        *self.done.borrow_mut() += 1;
    }

    fn show_progress(&self) {
        print!("\r{}", self);
        io::stdout().flush().unwrap();
    }
}

type Gallery = (String, (Option<usize>, Option<usize>));

pub struct Crawler {
    client: Client,
    semaphore: Semaphore,
    progress: Progress,
}

impl Crawler {
    pub fn new(credential: Credential) -> Self {
        let cookie = format!(
            "ipb_member_id={}; ipb_pass_hash={}",
            credential.ipb_member_id, credential.ipb_pass_hash
        );
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            header::HeaderValue::from_str(&cookie).unwrap(),
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT)
            .timeout(Duration::new(TIMEOUT, 0))
            .build()
            .unwrap();
        Crawler {
            client,
            semaphore: Semaphore::new(CONCURRENCY),
            progress: Progress::new(),
        }
    }

    pub async fn crawl(&mut self, galleries: Vec<Gallery>) {
        for (url, (start, end)) in galleries {
            // Reset the progress.
            self.progress = Progress::new();

            // Request the first index page, determine the number of index pages,
            // and extract urls to image pages.
            let page = match self.request_page(&url, Vec::new()).await {
                Ok(resp) => resp.text().await.unwrap(),
                Err(err) => {
                    eprintln!("Fail to request the index page 1 at `{}`: {}", url, err);
                    continue;
                }
            };
            let document = Html::parse_document(&page);
            let title = self.extract_title(&document).unwrap();
            let page_num = self.extract_page_num(&document).unwrap();
            let mut image_page_urls = self.extract_image_page_urls(&document).unwrap();
            self.progress.set_title(&title);
            self.progress.show_progress();

            // Request other index pages and extract urls to image pages.
            let futures = (1..page_num)
                .into_iter()
                .map(|p| self.request_page(&url, vec![("p".to_string(), p.to_string())]));
            let results = future::join_all(futures).await;
            let mut failed = false;
            for (p, result) in results.into_iter().enumerate() {
                let page = match result {
                    Ok(resp) => resp.text().await.unwrap(),
                    Err(err) => {
                        eprintln!("Fail to request index page {}: {}", p + 1, err);
                        failed = true;
                        break;
                    }
                };
                let document = Html::parse_document(&page);
                let urls = self.extract_image_page_urls(&document).unwrap();
                image_page_urls.extend(urls);
            }
            if failed {
                continue;
            }

            // Select the appropriate range that we want to crawl.
            let len = image_page_urls.len();
            let range = match (start, end) {
                (Some(start), Some(end)) if start <= end && end <= len => start - 1..end,
                (None, None) => 0..len,
                _ => {
                    eprintln!("Invalid range `{:?}-{:?}`.", start, end);
                    continue;
                }
            };
            let mut images: Vec<_> = image_page_urls[range]
                .iter()
                .enumerate()
                .map(|(i, url)| (i, url, false, Vec::new()))
                .collect();
            self.progress.set_total(images.len());
            self.progress.show_progress();

            // Create the gallery directory.
            let title = sanitize_filename::sanitize(title);
            let path = PathBuf::from(format!("{}/{}", PATH, title));
            fs::create_dir(&path)
                .map_err(|err| format!("Fail to create the gallery directory: {}", err))
                .unwrap();

            // Enter the main crawling loop.
            for _ in 0..=RELOAD {
                let futures = images
                    .iter_mut()
                    .filter(|(_, _, crawled, _)| !*crawled)
                    .map(|(i, url, crawled, query)| {
                        self.crawl_image(i, url, crawled, query, &path)
                    });
                future::join_all(futures).await;
            }

            // Print failed images.
            let failed_images: Vec<_> = images
                .iter()
                .filter(|(_, _, crawled, _)| !*crawled)
                .collect();
            println!();
            if !failed_images.is_empty() {
                println!("Fail to crawl the following images:");
                let mut buffer = String::new();
                for (num, _, _, _) in failed_images {
                    buffer.push_str(&format!("{}, ", num + 1));
                }
                buffer.pop();
                buffer.pop();
                println!("{}", buffer);
            }
        }
    }

    async fn request_page(
        &self,
        url: &str,
        query: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        let _ = self.semaphore.acquire().await.unwrap();
        for r in 0..=RETRY {
            return match self.client.get(url).query(&query).send().await {
                Ok(resp) => Ok(resp),
                Err(err) if r == RETRY => Err(err),
                _ => continue,
            };
        }
        unreachable!()
    }

    fn extract_title(&self, document: &Html) -> Result<String, String> {
        let japanese_title = document
            .select(&JAPANESE_TITLE_SELECTOR)
            .next()
            .ok_or("Fail to locate the Japanese title.")?
            .inner_html();
        if !japanese_title.is_empty() {
            Ok(japanese_title)
        } else {
            let default_title = document
                .select(&DEFAULT_TITLE_SELECTOR)
                .next()
                .ok_or("Fail to locate the default title.")?
                .inner_html();
            Ok(default_title)
        }
    }

    fn extract_page_num(&self, document: &Html) -> Result<u16, String> {
        document
            .select(&PAGE_NUM_SELECTOR)
            .next()
            .ok_or("Fail to locate the page number.")?
            .inner_html()
            .parse()
            .map_err(|err| format!("Fail to parse the page number: {}", err))
    }

    fn extract_image_page_urls(&self, document: &Html) -> Result<Vec<String>, String> {
        let a_tags: Vec<_> = document.select(&IMAGE_PAGE_SELECTOR).collect();
        if a_tags.is_empty() {
            Err("Fail to locate image pages urls.".into())
        } else {
            let mut urls = Vec::new();
            for a_tag in a_tags {
                let url = a_tag
                    .value()
                    .attr("href")
                    .ok_or("No `href` in links to image pages.")?
                    .to_string();
                urls.push(url);
            }
            Ok(urls)
        }
    }

    async fn crawl_image(
        &self,
        i: &usize,
        url: &str,
        crawled: &mut bool,
        query: &mut Vec<(String, String)>,
        path: &Path,
    ) {
        // Request the image page.
        let result = self.request_page(url, query.to_vec()).await;
        let page = match result {
            Ok(resp) => resp.text().await.unwrap(),
            Err(_) => return,
        };
        let document = Html::parse_document(&page);
        let (image_url, new_query) = self.extract_image_urls(&document).unwrap();
        query.push(new_query);

        // Crawl the image.
        if let Ok(resp) = self.request_page(&image_url, Vec::new()).await {
            if let Ok(image) = resp.bytes().await {
                let file_name = self.build_file_name(i, &image_url);
                let mut file = fs::File::create(path.join(file_name))
                    .map_err(|err| format!("Fail to create the image file: {}", err))
                    .unwrap();
                file.write_all(&image).expect("Fail to write the image.");
                *crawled = true;
                self.progress.make_progress();
                self.progress.show_progress();
            }
        }
    }

    fn extract_image_urls(&self, document: &Html) -> Result<(String, (String, String)), String> {
        // Extract the image url.
        let image_url = document
            .select(&IMAGE_SELECTOR)
            .next()
            .ok_or("Fail to locate the image.")?
            .value()
            .attr("src")
            .ok_or("No `src` in the link to the image.")
            .map(|attr| attr.to_string())?;

        // Extract the reload parameters.
        let reload_fn = document
            .select(&RELOAD_SELECTOR)
            .next()
            .ok_or("Fail to locate the reloading function.")?
            .value()
            .attr("onclick")
            .ok_or("No `onclick` in the reloading function.")
            .map(|attr| attr.to_string())?;
        let caps = RELOAD_REGEX
            .captures(&reload_fn)
            .ok_or(format!("Invalid reloading function: {}", reload_fn))?;

        Ok((image_url, (caps[1].to_string(), caps[2].to_string())))
    }

    fn build_file_name(&self, i: &usize, url: &str) -> String {
        let caps = FILE_NAME_EXTENSION_REGEX
            .captures(url)
            .ok_or(format!("Invalid file name extension: {}", url))
            .unwrap();
        format!("{:0>4}{}", i + 1, &caps[0])
    }
}
