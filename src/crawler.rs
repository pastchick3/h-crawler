use futures::future;
use log::{debug, error, info, warn};
use reqwest::header;
use reqwest::{Client, Error};
use scraper::{Html, Selector};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time;

use crate::error::DisplayableError;

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);
const TIMEOUT: u64 = 30; // request timeout (in second)
const DELAY: u64 = 2; // delay after each request (in second)
const RETRY: u8 = 2;
const CONCURRENCY: usize = 3;

const PAGE_NUM_SELECTOR: &str = "#asm + div td:nth-last-child(2) > a";
const IMAGE_PAGE_SELECTOR: &str = "#gdt a";
const IMAGE_SELECTOR: &str = "#img";

pub struct Crawler {
    client: Client,
    semaphore: Semaphore,
}

impl Crawler {
    pub fn new(ipb_member_id: &str, ipb_pass_hash: &str) -> Result<Self, DisplayableError> {
        let semaphore = Semaphore::new(CONCURRENCY);

        let cookie = format!(
            "ipb_member_id={}; ipb_pass_hash={}",
            ipb_member_id, ipb_pass_hash
        );
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            header::HeaderValue::from_str(&cookie)
                .map_err(|_| format!("Invalid cookie: {}", cookie))?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT)
            .timeout(Duration::new(TIMEOUT, 0))
            .build()?;

        Ok(Crawler { client, semaphore })
    }

    pub async fn crawl(
        &self,
        artist: &str,
        title: &str,
        url: &str,
        start: Option<u16>,
        end: Option<u16>,
    ) -> Result<(), DisplayableError> {
        // Request the first index page and determine there are how many index pages.
        let page = self.request_page(url).await?;
        let page_num = self.extract_page_num(&page)?;
        let mut image_page_urls = self.extract_image_page_urls(&page)?;

        // Request other index pages.
        for num in 1..page_num {
            let url = format!("{}?p={}", url, num);
            let page = self.request_page(&url).await?;
            let urls = self.extract_image_page_urls(&page)?;
            image_page_urls.extend(urls);
        }

        // Choose the appropriate range.
        let range = match (start, end) {
            (Some(start), Some(end)) => (start - 1) as usize..end as usize,
            (Some(start), None) => (start - 1) as usize..image_page_urls.len(),
            (None, Some(end)) => 0..end as usize,
            (None, None) => 0..image_page_urls.len(),
        };

        let mut failed_pages = Vec::new();
        // Request for img pages.
        let futures = image_page_urls[range]
            .iter()
            .map(|url| self.request_page(url));
        let results = future::join_all(futures).await;
        let mut image_urls = Vec::new();
        for (i, result) in results.iter().enumerate() {
            if let Ok(page) = result {
                let url = self.extract_image_urls(&page)?;
                image_urls.push((i + 1, url));
            } else {
                failed_pages.push(i + 1);
            }
        }

        // Download images.
        let path = PathBuf::from(format!("./[{}] {}", artist, title));
        fs::create_dir(&path)?;
        let futures = image_urls
            .iter()
            .map(|(i, url)| self.request_image(*i, url));
        let results = future::join_all(futures).await;

        for result in results {
            let (image_num, bytes) = result?;
            let filename = format!("{}.jpg", image_num);
            let mut file = fs::File::create(path.join(filename))?;
            file.write_all(&bytes)?;
        }

        Ok(())
    }

    async fn request_page(&self, url: &str) -> Result<String, Error> {
        let _ = self.semaphore.acquire().await;
        let mut retry = 0;
        loop {
            let result = self._request_page(url).await;
            if let Ok(page) = result {
                return Ok(page);
            }
            if retry == RETRY {
                return result;
            }
            retry += 1;
            time::delay_for(Duration::from_secs(DELAY)).await;
        }
    }

    async fn _request_page(&self, url: &str) -> Result<String, Error> {
        self.client.get(url).send().await?.text().await
    }

    fn extract_page_num(&self, page: &str) -> Result<u16, String> {
        let document = Html::parse_document(page);
        let selector = Selector::parse(PAGE_NUM_SELECTOR).unwrap();
        document
            .select(&selector)
            .next()
            .ok_or_else(|| String::from("Can not find the page number."))?
            .inner_html()
            .parse()
            .map_err(|err| format!("Can not parse the page number: {}", err))
    }

    fn extract_image_page_urls(&self, page: &str) -> Result<Vec<String>, String> {
        let document = Html::parse_document(page);
        let selector = Selector::parse(IMAGE_PAGE_SELECTOR).unwrap();
        let a_tags: Vec<_> = document.select(&selector).collect();
        if a_tags.is_empty() {
            Err(String::from("Can not find urls for image pages."))
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

    fn extract_image_urls(&self, page: &str) -> Result<String, String> {
        let document = Html::parse_document(&page);
        let selector = Selector::parse(IMAGE_SELECTOR).unwrap();
        document
            .select(&selector)
            .next()
            .ok_or_else(|| String::from("Can not find the image."))?
            .value()
            .attr("src")
            .map(|attr| attr.to_string())
            .ok_or_else(|| String::from("No `src` in links to images."))
    }

    async fn request_image(&self, image_num: usize, url: &str) -> Result<(usize, Vec<u8>), Error> {
        let _ = self.semaphore.acquire().await;
        let mut retry = 0;
        loop {
            let result = self._request_image(url).await;
            match result {
                Ok(image) => {
                    return Ok((image_num, image));
                }
                Err(err) if retry == RETRY => {
                    return Err(err);
                }
                Err(_) => {
                    retry += 1;
                    time::delay_for(Duration::from_secs(DELAY)).await;
                }
            }
        }
    }

    async fn _request_image(&self, url: &str) -> Result<Vec<u8>, Error> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes.into_iter().collect())
    }
}
