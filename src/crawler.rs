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
    static ref JAPANESE_TITLE_SELECTOR: Selector = Selector::parse(r"#gj").unwrap();
    static ref DEFAULT_TITLE_SELECTOR: Selector = Selector::parse(r"#gn").unwrap();
    static ref IMAGE_COUNT_SELECTOR: Selector =
        Selector::parse(r"#gdd tr:nth-child(6) td:nth-child(2)").unwrap();
    static ref IMAGE_COUNT_REGEX: Regex = Regex::new(r"(\d+) .+").unwrap();
    static ref IMAGE_PAGE_SELECTOR: Selector = Selector::parse(r"#gdt a").unwrap();
    static ref IMAGE_SELECTOR: Selector = Selector::parse(r"#img").unwrap();
    static ref FILE_EXTENSION_REGEX: Regex = Regex::new(r"\.[^\.]+?$").unwrap();
    static ref RELOAD_SELECTOR: Selector = Selector::parse(r"#loadfail").unwrap();
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

pub struct Crawler {
    client: Client,
    semaphore: Semaphore,
    progress: Progress,
    verbose: bool,
}

impl Crawler {
    pub fn new(credential: Credential, verbose: bool) -> Self {
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
            progress: Progress::new("EH Crawler", 0),
            verbose,
        }
    }

    pub async fn crawl(&mut self, (url, range): (String, Option<(usize, usize)>)) {
        // Crawl the home page and extract some basic information.
        let page = match self.request_page(&url, Vec::new()).await {
            Ok(page) => page,
            Err(err) => {
                eprintln!("Fail to request the index page 1 for `{}`: {}", url, err);
                return;
            }
        };
        let document = Html::parse_document(&page);
        let title = self.extract_title(&document).unwrap();
        let image_count = self.extract_image_count(&document).unwrap();

        // Determine a proper range.
        let (start, end) = match range {
            Some((start, end)) => (start, end),
            None => (1, image_count),
        };

        self.progress = Progress::new(&title, end - start + 1);
        if !self.verbose {
            self.progress.print_progress();
        }

        let start_page = start / 20;
        let start = start % 20 - 1;
        let end_page = end / 20;
        let end = (end_page - start_page) * 20 + end % 20;

        if self.verbose {
            eprintln!(
                "Crawl `{}` from {}:{} to {}:{} at `{}`.",
                title, start_page, start, end_page, end, url
            );
        }

        // Crawl index pages and extract links to image pages.
        let mut image_page_urls = Vec::new();
        let futures = (start_page..=end_page)
            .into_iter()
            .map(|p| self.request_page(&url, vec![("p".to_string(), p.to_string())]));
        let results = future::join_all(futures).await;
        for (p, result) in results.into_iter().enumerate() {
            let page = match result {
                Ok(page) => page,
                Err(err) => {
                    eprintln!(
                        "Fail to request the index page {} for `{}`: {}",
                        p + 1,
                        url,
                        err
                    );
                    return;
                }
            };
            let document = Html::parse_document(&page);
            let urls = self.extract_image_page_urls(&document).unwrap();
            image_page_urls.extend(urls);
        }

        // Create the gallery directory.
        let title = sanitize_filename::sanitize(title);
        let path = PathBuf::from(format!("{}/{}", PATH, title));
        fs::create_dir(&path)
            .map_err(|err| {
                format!(
                    "Fail to create the gallery directory for `{}`: {}",
                    title, err
                )
            })
            .unwrap();

        // Crawl image pages and images.
        let mut image_pages: Vec<_> = image_page_urls[start..end]
            .iter()
            .enumerate()
            .map(|(i, url)| (i, url, false, Vec::new()))
            .collect();
        for _ in 0..=RELOAD {
            let futures = image_pages
                .iter_mut()
                .filter(|(_, _, crawled, _)| !*crawled)
                .map(|(i, url, crawled, query)| self.crawl_image(i, url, crawled, query, &path));
            future::join_all(futures).await;
        }

        // Print failed images.
        if !self.verbose {
            println!();
        }
        let failed_images: Vec<_> = image_pages
            .iter()
            .filter(|(_, _, crawled, _)| !*crawled)
            .collect();
        if !failed_images.is_empty() {
            println!("Fail to crawl the following images:");
            let mut buffer = String::new();
            for (i, _, _, _) in failed_images {
                buffer.push_str(&format!("{}, ", i + 1));
            }
            buffer.pop();
            buffer.pop();
            println!("{}", buffer);
        }
    }

    async fn request_page(&self, url: &str, query: Vec<(String, String)>) -> Result<String, Error> {
        match self.request_url(url, query).await {
            Ok(resp) => Ok(resp.text().await.unwrap()),
            Err(err) => Err(err),
        }
    }

    async fn request_url(
        &self,
        url: &str,
        query: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        let _permit = self.semaphore.acquire().await.unwrap();
        for r in 0..=RETRY {
            return match self.client.get(url).query(&query).send().await {
                Ok(resp) => {
                    if self.verbose {
                        let req = self.client.get(url).query(&query).build().unwrap();
                        eprintln!("Request `{}` (retry={}) succeeds.", req.url(), r);
                    }
                    Ok(resp)
                }
                Err(err) => {
                    if self.verbose {
                        let req = self.client.get(url).query(&query).build().unwrap();
                        eprintln!("Request `{}` (retry={}) fails.", req.url(), r);
                    }
                    if r == RETRY {
                        Err(err)
                    } else {
                        continue;
                    }
                }
            };
        }
        unreachable!();
    }

    fn extract_title(&self, document: &Html) -> Result<String, String> {
        let japanese_title = document
            .select(&JAPANESE_TITLE_SELECTOR)
            .next()
            .ok_or("Fail to locate the Japanese title.")?
            .inner_html();
        if japanese_title.is_empty() {
            let default_title = document
                .select(&DEFAULT_TITLE_SELECTOR)
                .next()
                .ok_or("Fail to locate the default title.")?
                .inner_html();
            Ok(default_title)
        } else {
            Ok(japanese_title)
        }
    }

    fn extract_image_count(&self, document: &Html) -> Result<usize, String> {
        let length_field = document
            .select(&IMAGE_COUNT_SELECTOR)
            .next()
            .ok_or("Fail to locate the image count.")?
            .inner_html();
        let caps = IMAGE_COUNT_REGEX
            .captures(&length_field)
            .ok_or(format!("Invalid image count `{}`.", length_field))?;
        caps[1]
            .to_string()
            .parse()
            .map_err(|err| format!("Fail to parse the image count: {}", err))
    }

    fn extract_image_page_urls(&self, document: &Html) -> Result<Vec<String>, String> {
        let a_tags: Vec<_> = document.select(&IMAGE_PAGE_SELECTOR).collect();
        if a_tags.is_empty() {
            Err(String::from("Fail to locate image pages urls."))
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
        // Crawl the image page.
        let page = match self.request_page(url, query.to_vec()).await {
            Ok(page) => page,
            Err(_) => return,
        };
        let document = Html::parse_document(&page);
        let (image_url, new_query) = self.extract_image_urls(&document).unwrap();
        query.push(new_query);

        // Crawl the image.
        if let Ok(resp) = self.request_url(&image_url, Vec::new()).await {
            if let Ok(image) = resp.bytes().await {
                let file_name = self.build_file_name(i, &image_url);
                let mut file = fs::File::create(path.join(file_name))
                    .map_err(|err| format!("Fail to create the image file: {}", err))
                    .unwrap();
                file.write_all(&image).expect("Fail to write the image.");
                *crawled = true;
                self.progress.make_progress();
                if !self.verbose {
                    self.progress.print_progress();
                }
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
        let caps = FILE_EXTENSION_REGEX
            .captures(url)
            .ok_or(format!("Invalid file extension: {}", url))
            .unwrap();
        format!("{:0>4}{}", i + 1, &caps[0])
    }
}
