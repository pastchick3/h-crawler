use futures::future;
use log::{debug, info};
use regex::Regex;
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
const TIMEOUT: u64 = 20;
const RETRY: usize = 1;
const CONCURRENCY: usize = 5;
const PAGE_NUM_SELECTOR: &str = "#asm + div td:nth-last-child(2) > a";
const IMAGE_PAGE_SELECTOR: &str = "#gdt a";
const IMAGE_SELECTOR: &str = "#img";
const RELOAD_SELECTOR: &str = "#loadfail";
const RELOAD_REGEX: &str = r"return (.+?)\('(.+?)'\)";

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
        let page = self.request_page(url, &None).await?;
        let page_num = self.extract_page_num(&page)?;
        let mut image_page_urls = self.extract_image_page_urls(&page)?;
        info!("Find {} index page(s).", page_num);

        // Request other index pages and extract urls to image pages.
        for num in 1..page_num {
            let page = self
                .request_page(&url, &Some((String::from("p"), format!("{}", num))))
                .await?;
            let urls = self.extract_image_page_urls(&page)?;
            image_page_urls.extend(urls);
        }
        info!("Find {} image(s).", image_page_urls.len());

        // Select the appropriate range that we want to download.
        if let Some(start) = start {
            if start as usize > image_page_urls.len() {
                return Err(DisplayableError::from("Invalid range."));
            }
        }
        if let Some(end) = end {
            if end as usize > image_page_urls.len() {
                return Err(DisplayableError::from("Invalid range."));
            }
        }
        let range = match (start, end) {
            (Some(start), Some(end)) => (start - 1) as usize..end as usize,
            (Some(start), None) => (start - 1) as usize..image_page_urls.len(),
            (None, Some(end)) => 0..end as usize,
            (None, None) => 0..image_page_urls.len(),
        };
        let mut images: Vec<_> = image_page_urls[range]
            .iter()
            .enumerate()
            .map(|(i, url)| (i + 1, url, None, None))
            .collect();
        info!("Select {} image(s).", images.len());

        // Create the gallery directory.
        let path = PathBuf::from(format!("{}/[{}] {}", PATH, artist, title));
        fs::create_dir(&path)
            .map_err(|err| format!("Can not create the gallery directory: {}", err))?;

        // Enter the main downloading loop.
        for retry in 0..=RETRY {
            let futures = images
                .iter_mut()
                .map(|(i, u, b, r)| self._crawl(i, u, b, r, retry));
            let results = future::join_all(futures).await;
            for result in results {
                result?;
            }
        }

        // Write images to the local disk and return failed images.
        let mut failed_images = Vec::new();
        for (image_num, _, bytes, _) in images {
            if let Some(bytes) = bytes {
                let filename = format!("{}.jpg", image_num);
                let mut file = fs::File::create(path.join(filename))?;
                file.write_all(&bytes)?;
            } else {
                failed_images.push(image_num);
            }
        }
        Ok(failed_images)
    }

    async fn _crawl(
        &self,
        image_num: &usize,
        url: &str,
        bytes: &mut Option<Vec<u8>>,
        reload: &mut Option<(String, String)>,
        retry: usize,
    ) -> Result<(), DisplayableError> {
        // Return if this image has been downloaded.
        if bytes.is_some() {
            return Ok(());
        }

        // Request for image pages.
        let result = self.request_page(url, &Option::clone(reload)).await;
        let page = if let Ok(page) = result {
            page
        } else {
            return Ok(());
        };
        let (image_url, new_reload) = self.extract_image_urls(&page)?;
        *reload = Some(new_reload);

        // Download images.
        let result = self.request_image(&image_url).await;
        let mut status = "fails";
        if let Ok(image) = result {
            *bytes = Some(image);
            status = "succeeds";
        }

        // Print the log.
        if retry == 0 {
            info!("Downloading image {} {}.", image_num, status);
        } else {
            info!(
                "Downloading image {} (retry {}) {}.",
                image_num, retry, status
            );
        }

        Ok(())
    }

    async fn request_page(
        &self,
        url: &str,
        query: &Option<(String, String)>,
    ) -> Result<String, Error> {
        let _ = self.semaphore.acquire().await;
        let mut retry = 0;
        loop {
            let result = self._request_page(url, query).await;
            match result {
                Ok(page) => {
                    if retry == 0 {
                        debug!("Requesting page `{}` succeeds.", url);
                    } else {
                        debug!("Requesting page `{} (retry {}) succeeds.", url, retry);
                    }
                    return Ok(page);
                }
                Err(err) if retry == RETRY => {
                    if retry == 0 {
                        debug!("Requesting page `{}` fails.", url);
                    } else {
                        debug!("Requesting page `{} (retry {}) fails.", url, retry);
                    }
                    return Err(err);
                }
                Err(_) => {
                    if retry == 0 {
                        debug!("Requesting page `{}` fails.", url);
                    } else {
                        debug!("Requesting page `{} (retry {}) fails.", url, retry);
                    }
                    retry += 1;
                }
            }
        }
    }

    async fn _request_page(
        &self,
        url: &str,
        query: &Option<(String, String)>,
    ) -> Result<String, Error> {
        let mut request = self.client.get(url);
        if let Some(query) = query {
            request = request.query(&[query]);
        }
        request.send().await?.text().await
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

    fn extract_image_urls(&self, page: &str) -> Result<(String, (String, String)), String> {
        let document = Html::parse_document(&page);

        // Extract image urls.
        let image_selector = Selector::parse(IMAGE_SELECTOR).unwrap();
        let image_url = document
            .select(&image_selector)
            .next()
            .ok_or_else(|| String::from("Can not find the image."))?
            .value()
            .attr("src")
            .ok_or_else(|| String::from("No `src` in the link to the image."))
            .map(|attr| attr.to_string())?;

        // Extract reload parameters.
        let reload_selector = Selector::parse(RELOAD_SELECTOR).unwrap();
        let reload_fn = document
            .select(&reload_selector)
            .next()
            .ok_or_else(|| String::from("Can not find the reloading function."))?
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
                    if retry == 0 {
                        debug!("Requesting image `{}` succeeds.", url);
                    } else {
                        debug!("Requesting image `{} (retry {}) succeeds.", url, retry);
                    }
                    return Ok(image);
                }
                Err(err) if retry == RETRY => {
                    if retry == 0 {
                        debug!("Requesting image `{}` fails.", url);
                    } else {
                        debug!("Requesting image `{} (retry {}) fails.", url, retry);
                    }
                    return Err(err);
                }
                Err(_) => {
                    if retry == 0 {
                        debug!("Requesting image `{}` fails.", url);
                    } else {
                        debug!("Requesting image `{} (retry {}) fails.", url, retry);
                    }
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
