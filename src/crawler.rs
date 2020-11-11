use crate::Credential;
use futures::future;
use regex::Regex;
use reqwest::header;
use reqwest::{Client, Error};
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

const DEFAULT_TITLE_SELECTOR: &str = "#gn";
const JAPANESE_TITLE_SELECTOR: &str = "#gj";
const PAGE_NUM_SELECTOR: &str = "#asm + div td:nth-last-child(2) > a";
const IMAGE_PAGE_SELECTOR: &str = "#gdt a";
const FILE_NAME_EXTENSION_REGEX: &str = r"\.[^\.]+?$";
const IMAGE_SELECTOR: &str = "#img";
const RELOAD_SELECTOR: &str = "#loadfail";
const RELOAD_REGEX: &str = r"return (.+?)\('(.+?)'\)";

struct Progress {
    title: RefCell<String>,
    done: RefCell<usize>,
    total: RefCell<usize>,
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{} => {}/{}",
            self.title.borrow(),
            self.done.borrow(),
            self.total.borrow()
        )
    }
}

impl Progress {
    fn new() -> Self {
        Progress {
            title: RefCell::new("_".to_string()),
            done: RefCell::new(0),
            total: RefCell::new(0),
        }
    }

    fn set_title(&self, title: &str) {
        *self.title.borrow_mut() = title.to_string();
    }

    fn set_total(&self, total: usize) {
        *self.total.borrow_mut() = total;
    }

    fn make_progress(&self) {
        *self.done.borrow_mut() += 1;
    }

    fn show_progress(&self) {
        print!("\r{}", self);
        io::stdout().flush().unwrap();
    }
}

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

    pub async fn crawl(&self, url: &str, start: Option<usize>, end: Option<usize>) {
        // Request the first index page, determine the number of index pages,
        // and extract urls to image pages.
        let page = match self.request_page(url, &Vec::new()).await {
            Ok(page) => page,
            Err(err) => panic!("Fail to request index page 1: {}", err),
        };
        let title = self.extract_title(&page).unwrap();
        let page_num = self.extract_page_num(&page).unwrap();
        let mut image_page_urls = self.extract_image_page_urls(&page).unwrap();
        self.progress.set_title(&title);
        self.progress.show_progress();

        // Request other index pages and extract urls to image pages.
        for num in 1..page_num {
            let result = self
                .request_page(&url, &[(String::from("p"), format!("{}", num))])
                .await;
            let page = match result {
                Ok(page) => page,
                Err(err) => panic!("Fail to request index page {}: {}", num + 1, err),
            };
            let urls = self.extract_image_page_urls(&page).unwrap();
            image_page_urls.extend(urls);
        }

        // Select the appropriate range that we want to crawl.
        let len = image_page_urls.len();
        let range = match (start, end) {
            (Some(start), Some(end)) => {
                if start > end || end > len {
                    panic!("Invalid range.");
                }
                start - 1..end
            }
            (Some(start), None) => {
                if start > len {
                    panic!("Invalid range.");
                }
                start - 1..len
            }
            (None, Some(end)) => {
                if end > len {
                    panic!("Invalid range.");
                }
                0..end
            }
            (None, None) => 0..len,
        };
        let mut images: Vec<_> = image_page_urls[range]
            .iter()
            .enumerate()
            .map(|(i, url)| (i, url, false, Vec::new()))
            .collect();
        self.progress.set_total(images.len());
        self.progress.show_progress();

        // Create the gallery directory.
        let path = PathBuf::from(format!("{}/{}", PATH, title));
        fs::create_dir(&path)
            .map_err(|err| format!("Fail to create the gallery directory: {}", err))
            .unwrap();

        // Enter the main crawling loop.
        for _ in 0..=RELOAD {
            let futures = images
                .iter_mut()
                .map(|(num, url, crawled, query)| self._crawl(num, url, crawled, query, &path));
            future::join_all(futures).await;
        }

        // Print failed images.
        let failed_images: Vec<_> = images
            .iter()
            .filter(|(_, _, crawled, _)| !*crawled)
            .collect();
        if !failed_images.is_empty() {
            println!();
            println!("Fail to crawl the following images:");
            let mut buffer = String::new();
            for (num, _, _, _) in failed_images {
                buffer.push_str(&format!("{}, ", num+1));
            }
            buffer.pop();
            buffer.pop();
            println!("{}", buffer);
        }
    }

    async fn request_page(&self, url: &str, query: &[(String, String)]) -> Result<String, Error> {
        let _ = self.semaphore.acquire().await;
        let mut retry = 0;
        loop {
            let result = self._request_page(url, query).await;
            match result {
                Ok(page) => {
                    return Ok(page);
                }
                Err(err) if retry == RETRY => {
                    return Err(err);
                }
                Err(_) => {
                    retry += 1;
                }
            }
        }
    }

    async fn _request_page(&self, url: &str, query: &[(String, String)]) -> Result<String, Error> {
        self.client.get(url).query(query).send().await?.text().await
    }

    fn extract_title(&self, page: &str) -> Result<String, String> {
        let document = Html::parse_document(page);
        let japanese_title_selector = Selector::parse(JAPANESE_TITLE_SELECTOR).unwrap();
        let japanese_title = document
            .select(&japanese_title_selector)
            .next()
            .ok_or_else(|| String::from("Fail to locate the Japanese title."))?
            .inner_html();
        if japanese_title.is_empty() {
            let default_title_selector = Selector::parse(DEFAULT_TITLE_SELECTOR).unwrap();
            let default_title = document
                .select(&default_title_selector)
                .next()
                .ok_or_else(|| String::from("Fail to locate the default title."))?
                .inner_html();
            Ok(default_title)
        } else {
            Ok(japanese_title)
        }
    }

    fn extract_page_num(&self, page: &str) -> Result<u16, String> {
        let document = Html::parse_document(page);
        let selector = Selector::parse(PAGE_NUM_SELECTOR).unwrap();
        document
            .select(&selector)
            .next()
            .ok_or_else(|| String::from("Fail to locate the page number."))?
            .inner_html()
            .parse()
            .map_err(|err| format!("Fail to parse the page number: {}", err))
    }

    fn extract_image_page_urls(&self, page: &str) -> Result<Vec<String>, String> {
        let document = Html::parse_document(page);
        let selector = Selector::parse(IMAGE_PAGE_SELECTOR).unwrap();
        let a_tags: Vec<_> = document.select(&selector).collect();
        if a_tags.is_empty() {
            Err(String::from("Fail to locate image pages urls."))
        } else {
            let mut urls = Vec::new();
            for a_tag in a_tags {
                let url = a_tag
                    .value()
                    .attr("href")
                    .ok_or_else(|| String::from("No `href` in links to image pages."))?
                    .to_string();
                urls.push(url);
            }
            Ok(urls)
        }
    }

    fn build_file_name(&self, image_num: &usize, url: &str) -> String {
        let re = Regex::new(FILE_NAME_EXTENSION_REGEX).unwrap();
        let caps = re
            .captures(url)
            .ok_or_else(|| format!("Invalid file name extension: {}", url))
            .unwrap();
        format!("{:0>4}{}", image_num + 1, caps[0].to_string())
    }

    async fn _crawl(
        &self,
        num: &usize,
        url: &str,
        crawled: &mut bool,
        query: &mut Vec<(String, String)>,
        path: &Path,
    ) {
        // Return imediately if this image has been crawled.
        if *crawled {
            return;
        }

        // Request for the image page.
        let result = self.request_page(url, query).await;
        let page = match result {
            Ok(page) => page,
            Err(_) => return,
        };
        let (image_url, new_query) = self.extract_image_urls(&page).unwrap();
        query.push(new_query);

        // Crawl the image.
        let result = self.request_image(&image_url).await;
        if let Ok(image) = result {
            let file_name = self.build_file_name(num, &image_url);
            let mut file = fs::File::create(path.join(file_name))
                .map_err(|err| format!("Fail to create the image file: {}", err))
                .unwrap();
            file.write_all(&image).expect("Fail to write the image.");
            *crawled = true;
            self.progress.make_progress();
            self.progress.show_progress();
        }
    }

    fn extract_image_urls(&self, page: &str) -> Result<(String, (String, String)), String> {
        let document = Html::parse_document(&page);

        // Extract the image url.
        let image_selector = Selector::parse(IMAGE_SELECTOR).unwrap();
        let image_url = document
            .select(&image_selector)
            .next()
            .ok_or_else(|| String::from("Fail to locate the image."))?
            .value()
            .attr("src")
            .ok_or_else(|| String::from("No `src` in the link to the image."))
            .map(|attr| attr.to_string())?;

        // Extract the reload parameters.
        let reload_selector = Selector::parse(RELOAD_SELECTOR).unwrap();
        let reload_fn = document
            .select(&reload_selector)
            .next()
            .ok_or_else(|| String::from("Fail to locate the reloading function."))?
            .value()
            .attr("onclick")
            .ok_or_else(|| String::from("No `onclick` in the reloading function."))
            .map(|attr| attr.to_string())?;
        let re = Regex::new(RELOAD_REGEX).unwrap();
        let caps = re
            .captures(&reload_fn)
            .ok_or_else(|| format!("Invalid reloading function: {}", reload_fn))?;

        Ok((image_url, (caps[1].to_string(), caps[2].to_string())))
    }

    async fn request_image(&self, url: &str) -> Result<Vec<u8>, Error> {
        let _ = self.semaphore.acquire().await;
        let mut retry = 0;
        loop {
            let result = self._request_image(url).await;
            match result {
                Ok(image) => {
                    return Ok(image);
                }
                Err(err) if retry == RETRY => {
                    return Err(err);
                }
                Err(_) => {
                    retry += 1;
                }
            }
        }
    }

    async fn _request_image(&self, url: &str) -> Result<Vec<u8>, Error> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes.into_iter().collect())
    }
}
