use std::collections::HashMap;
use std::time::Duration;

use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::time;

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);
const EH_URL: &str = "https://e-hentai.org/";
const EX_URL: &str = "https://exhentai.org/";
const API_URL: &str = "https://api.e-hentai.org/api.php";
const TIMEOUT: u64 = 30;
const INTERVAL: u64 = 2;
const GALLERIES_PER_PAGE: usize = 25;
const GALLERY_COUNT_SELECTOR: &str = "div.ido div:nth-child(2) p";
const GALLERY_COUNT_REGEX: &str = r"(\d|,)+";
const GALLERY_SELECTOR: &str = "div.ido table.itg.gltc td.gl3c.glname a";
const GALLERY_REGEX: &str = r"/([^/]+)/([^/]+)/$";

bitflags! {
    #[derive(Default)]
    struct Category: u16 {
        const DOUJINSHI = 0b0000000010;
        const MANGA = 0b0000000100;
        const ARTIST_CG = 0b0000001000;
        const GAME_CG = 0b0000010000;
        const WESTERN = 0b1000000000;
        const NON_H = 0b0100000000;
        const IMAGE_SET = 0b0000100000;
        const COSPLAY = 0b0001000000;
        const ASIAN_PORN = 0b0010000000;
        const MISC = 0b0000000001;
    }
}

impl From<Category> for String {
    fn from(category: Category) -> Self {
        category.bits.to_string()
    }
}

type QueryResult<T> = Result<T, String>;
type Gallery = (u64, String);

#[derive(Serialize)]
struct ApiRequest {
    method: String,
    gidlist: Vec<Gallery>,
    namespace: u64,
}

impl ApiRequest {
    fn new<T: Iterator<Item = Gallery>>(galleries: T) -> Self {
        ApiRequest {
            method: String::from("gdata"),
            gidlist: galleries.collect(),
            namespace: 1,
        }
    }
}

#[derive(Deserialize)]
struct ApiResponse {
    gmetadata: Vec<Meta>,
}

#[derive(Deserialize)]
pub struct Meta {
    gid: u64,
    token: String,
    archiver_key: String,
    title: String,
    title_jpn: String,
    category: String,
    thumb: String,
    uploader: String,
    posted: String,
    filecount: String,
    filesize: u64,
    expunged: bool,
    rating: String,
    torrentcount: String,
    tags: Vec<String>,
}

pub struct GalleryPack {
    count: usize,
    data: Vec<Meta>,
}

pub struct Querier {
    client: Client,
    ex: bool,
    url: &'static str,
    username: Option<String>,
    password: Option<String>,
    gallery_count_selector: Selector,
    gallery_count_regex: Regex,
    gallery_selector: Selector,
    gallery_regex: Regex,
}

impl Querier {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .cookie_store(true)
            .timeout(Duration::new(TIMEOUT, 0))
            .build()
            .expect("Unable to initialize the request client.");
        Querier {
            client,
            ex: false,
            url: EH_URL,
            username: None,
            password: None,
            gallery_count_selector: Selector::parse(GALLERY_COUNT_SELECTOR).unwrap(),
            gallery_count_regex: Regex::new(GALLERY_COUNT_REGEX).unwrap(),
            gallery_selector: Selector::parse(GALLERY_SELECTOR).unwrap(),
            gallery_regex: Regex::new(GALLERY_REGEX).unwrap(),
        }
    }

    pub async fn new_ex(username: String, password: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .cookie_store(true)
            .timeout(Duration::new(TIMEOUT, 0))
            .build()
            .expect("Unable to initialize the request client.");

        let login_params = [
            ("UserName", &username),
            ("PassWord", &password),
            ("CookieDate", &String::from("1")),
        ];
        client
            .post("https://forums.e-hentai.org/index.php?act=Login&CODE=01")
            .form(&login_params)
            .send()
            .await
            .expect("Unable to log in to ExHentai.");

        Querier {
            client,
            ex: true,
            url: EX_URL,
            username: Some(username),
            password: Some(password),
            gallery_count_selector: Selector::parse(GALLERY_COUNT_SELECTOR).unwrap(),
            gallery_count_regex: Regex::new(GALLERY_COUNT_REGEX).unwrap(),
            gallery_selector: Selector::parse(GALLERY_SELECTOR).unwrap(),
            gallery_regex: Regex::new(GALLERY_REGEX).unwrap(),
        }
    }

    pub async fn query(
        &self,
        exhaustive: bool,
        params: HashMap<String, Vec<String>>,
    ) -> QueryResult<GalleryPack> {
        let mut category = String::from("0");
        let mut search = String::new();
        for (key, value) in params.iter() {
            match key.to_lowercase().as_str() {
                "category" => {
                    category = self.toggle_category(value)?;
                }
                "search" => {
                    for term in value {
                        let segment = format!("+{:?}", term);
                        search.push_str(&segment);
                    }
                }
                tag => {
                    for term in value {
                        let segment = format!("+{}:{:?}$", tag, term);
                        search.push_str(&segment);
                    }
                }
            }
        }

        let page = self.request_page(0, &category, &search).await?;
        let (page_count, mut galleries) = self.parse_page(&page)?;
        if exhaustive {
            for pg in 1..page_count {
                let page = self.request_page(pg, &category, &search).await?;
                let (_, g) = self.parse_page(&page)?;
                galleries.extend(g);
            }
        }

        self.request_meta(galleries).await
    }

    fn toggle_category(&self, categories: &[String]) -> QueryResult<String> {
        let mut category: Category = Default::default();
        for cat in categories {
            match cat.to_lowercase().as_str() {
                "doujinshi" => category |= Category::DOUJINSHI,
                "manga" => category |= Category::MANGA,
                "artist cg" => category |= Category::ARTIST_CG,
                "game cg" => category |= Category::GAME_CG,
                "western" => category |= Category::WESTERN,
                "non h" => category |= Category::NON_H,
                "image set" => category |= Category::IMAGE_SET,
                "cosplay" => category |= Category::COSPLAY,
                "asian porn" => category |= Category::ASIAN_PORN,
                "misc" => category |= Category::MISC,
                c => return Err(format!("Invalid category: {}", c)),
            }
        }
        Ok(String::from(!category))
    }

    async fn request_page(
        &self,
        page: u64,
        category: &str,
        search: &str,
    ) -> Result<String, String> {
        self.client
            .get(self.url)
            .query(&[("page", page)])
            .query(&[("f_cats", category)])
            .query(&[("f_search", search)])
            .send()
            .await
            .map_err(|err| format!("{}", err))?
            .text()
            .await
            .map_err(|err| format!("{}", err))
    }

    fn parse_page(&self, page: &str) -> QueryResult<(u64, Vec<Gallery>)> {
        let document = Html::parse_document(page);

        let gallery_count_str = document
            .select(&self.gallery_count_selector)
            .next()
            .ok_or(String::from("Unable to locate the gallery count."))?
            .inner_html();
        let gallery_count = self
            .gallery_count_regex
            .captures(&gallery_count_str)
            .ok_or(String::from("Unable to extract the gallery count."))?
            .get(0)
            .ok_or(String::from("Unable to extract the gallery count."))?
            .as_str()
            .replace(",", "")
            .parse::<u64>()
            .map_err(|_| String::from("Unable to extract the gallery count."))?;
        let page_count = (gallery_count as f64 / GALLERIES_PER_PAGE as f64).ceil() as u64;

        let gallery_links = document.select(&self.gallery_selector).collect::<Vec<_>>();
        if gallery_links.len() != GALLERIES_PER_PAGE {
            return Err(String::from("Unable to locate galleries."));
        }
        let mut galleries = Vec::new();
        for link in gallery_links {
            let href = link
                .value()
                .attr("href")
                .ok_or("Unable to extract the gallery link.")?;
            let caps = self
                .gallery_regex
                .captures(&href)
                .ok_or("Unable to extract the gallery link.")?;
            let id = caps
                .get(1)
                .ok_or("Unable to extract the gallery link.")?
                .as_str()
                .parse()
                .map_err(|_| String::from("Unable to extract the gallery link."))?;
            let token = caps
                .get(2)
                .ok_or("Unable to extract the gallery link.")?
                .as_str()
                .to_string();
            galleries.push((id, token));
        }

        Ok((page_count, galleries))
    }

    async fn request_meta(&self, galleries: Vec<Gallery>) -> QueryResult<GalleryPack> {
        let mut pack = GalleryPack {
            count: galleries.len(),
            data: Vec::new(),
        };
        for chunk in galleries.chunks(GALLERIES_PER_PAGE) {
            let params = ApiRequest::new(chunk.iter().map(|g| g.clone()));
            let meta = self
                .client
                .post(API_URL)
                .json(&params)
                .send()
                .await
                .map_err(|_| String::from("Unable to request gallery metadata."))?
                .json::<ApiResponse>()
                .await
                .map_err(|_| String::from("Unable to request gallery metadata."))?
                .gmetadata;
            pack.data.extend(meta);
            time::delay_for(Duration::from_secs(INTERVAL)).await;
        }
        Ok(pack)
    }
}
