use futures::future;
use log::info;
use reqwest::header;
use reqwest::{Client, Error};
use scraper::{Html, Selector};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::error::DisplayableError;

const PATH: &str = ".";
const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);
const TIMEOUT: u64 = 15;
const RETRY: usize = 2;
const CONCURRENCY: usize = 4;
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

        // We use these two cookies to log into the ExHentai.
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
    ) -> Result<Vec<usize>, DisplayableError> {
        // Request the first index page, determine the number of index pages,
        // and extract urls to image pages.
        let page = self.request_page(url).await?;
        let page_num = self.extract_page_num(&page)?;
        let mut image_page_urls = self.extract_image_page_urls(&page)?;

        // Request other index pages and extract urls to image pages.
        for num in 1..page_num {
            let url = format!("{}?p={}", url, num);
            let page = self.request_page(&url).await?;
            let urls = self.extract_image_page_urls(&page)?;
            image_page_urls.extend(urls);
        }

        // Determine the appropriate range we want to download.
        let range = match (start, end) {
            (Some(start), Some(end)) => (start - 1) as usize..end as usize,
            (Some(start), None) => (start - 1) as usize..image_page_urls.len(),
            (None, Some(end)) => 0..end as usize,
            (None, None) => 0..image_page_urls.len(),
        };

        // Create the gallery directory.
        let path = PathBuf::from(format!("{}/[{}] {}", PATH, artist, title));
        fs::create_dir(&path)
            .map_err(|err| format!("Can not create the gallery directory: {}", err))?;

        // Request for image pages.
        let futures = image_page_urls[range]
            .iter()
            .map(|url| self.request_page(url));
        let results = future::join_all(futures).await;
        let mut image_urls = Vec::new();
        let mut failed_images = Vec::new();
        for (index, result) in results.iter().enumerate() {
            if let Ok(page) = result {
                let url = self.extract_image_urls(&page)?;
                image_urls.push((index + 1, url));
            } else {
                failed_images.push(index + 1);
            }
        }

        // Download images.
        let futures = image_urls
            .iter()
            .map(|(page_num, url)| self.request_image(*page_num, url));
        let results = future::join_all(futures).await;
        for result in results {
            match result {
                Ok((image_num, bytes)) => {
                    let filename = format!("{}.jpg", image_num);
                    let mut file = fs::File::create(path.join(filename))?;
                    file.write_all(&bytes)?;
                }
                Err((image_num, _)) => {
                    failed_images.push(image_num);
                }
            }
        }

        Ok(failed_images)
    }

    async fn request_page(&self, url: &str) -> Result<String, Error> {
        let _ = self.semaphore.acquire().await;
        let mut retry = 0;
        loop {
            let result = self._request_page(url).await;
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
            Err(String::from("Can not find image pages urls."))
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
            .ok_or_else(|| String::from("No `src` in the link to the image."))
            .map(|attr| attr.to_string())
    }

    async fn request_image(
        &self,
        image_num: usize,
        url: &str,
    ) -> Result<(usize, Vec<u8>), (usize, Error)> {
        let _ = self.semaphore.acquire().await;
        let mut retry = 0;
        loop {
            let result = self._request_image(url).await;
            match result {
                Ok(image) => {
                    info!("Downloading image {} succeeds.", image_num);
                    return Ok((image_num, image));
                }
                Err(err) if retry == RETRY => {
                    info!("Downloading image {} fails.", image_num);
                    return Err((image_num, err));
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
