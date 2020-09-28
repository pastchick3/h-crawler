use std::sync::mpsc::{Receiver, Sender};
use std::path::PathBuf;
use log::{debug, error, info, warn};

use crate::gallery::Gallery;

pub struct Crawler {}

impl Crawler {
    pub fn new(username: String, password: String, resource: PathBuf, task_rx: Receiver<Gallery>, result_tx: Sender<Gallery>) -> Self {
        Crawler {}
    }
}

// //! A EH Metadata Querier
// //!
// //! This querier will first search EH to find all galleries that the user
// //! is interested in, then it will query the EH API to fetch metadata
// //! of these galleries.

// use std::collections::HashMap;
// use std::time::Duration;

// use regex::Regex;
// use reqwest::Client;
// use scraper::{Html, Selector};
// use serde::{Deserialize, Serialize};
// use tokio::time;

// const USER_AGENT: &str = concat!(
//     "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
//     "AppleWebKit/537.36 (KHTML, like Gecko) ",
//     "Chrome/81.0.4044.138 ",
//     "Safari/537.36 Edg/81.0.416.72",
// );

// const EH_URL: &str = "https://e-hentai.org/";
// const EX_URL: &str = "https://exhentai.org/";
// const API_URL: &str = "https://api.e-hentai.org/api.php";
// const TIMEOUT: u64 = 30; // request timeout (in second)
// const DELAY: u64 = 2; // delay after each request (in second)
// const GALLERIES_PER_PAGE: usize = 25;
// const ERROR_SELECTOR: &str = "#iw p";
// const GALLERY_COUNT_SELECTOR: &str = "div.ido div:nth-child(2) p.ip";
// const GALLERY_COUNT_REGEX: &str = r"(\d|,)+";
// const GALLERY_SELECTOR: &str = "div.ido table.itg.gltc td.gl3c.glname a";
// const GALLERY_REGEX: &str = r"/([^/]+)/([^/]+)/$";

// bitflags! {
//     /// EH uses bit masks to denote categories. There are 10 categories, and
//     /// each of them is represented as a single bit. When a user enters EH,
//     /// a variable is initialized to zero. Each time the user turns off a
//     /// category, the corresponding bit of this variable will be set The final
//     /// category number appearing in the query string is this variable interpreted
//     /// as an unsigned int.
//     #[derive(Default)]
//     struct Category: u16 {
//         const DOUJINSHI = 0b00_0000_0010;
//         const MANGA = 0b00_0000_0100;
//         const ARTIST_CG = 0b00_0000_1000;
//         const GAME_CG = 0b00_0001_0000;
//         const WESTERN = 0b10_0000_0000;
//         const NON_H = 0b01_0000_0000;
//         const IMAGE_SET = 0b00_0010_0000;
//         const COSPLAY = 0b00_0100_0000;
//         const ASIAN_PORN = 0b00_1000_0000;
//         const MISC = 0b00_0000_0001;
//     }
// }

// impl From<Category> for String {
//     fn from(category: Category) -> Self {
//         category.bits.to_string()
//     }
// }

// /// Generic result type used by the querier.
// type QuerierResult<T> = Result<T, String>;

// /// Each gallery is represented by `(gallery_id, gallery_token)`.
// /// See [EH API](https://ehwiki.org/wiki/API) for more information.
// type Gallery = (u32, String);

// /// Request body for EH API.
// /// See [EH API](https://ehwiki.org/wiki/API) for more information.
// #[derive(Serialize)]
// struct ApiRequest {
//     method: String,
//     gidlist: Vec<Gallery>,
//     namespace: u32,
// }

// impl ApiRequest {
//     fn new<T: Iterator<Item = Gallery>>(galleries: T) -> Self {
//         ApiRequest {
//             method: String::from("gdata"),
//             gidlist: galleries.collect(),
//             namespace: 1,
//         }
//     }
// }

// /// Response body for EH API.
// /// See [EH API](https://ehwiki.org/wiki/API) for more information.
// #[derive(Deserialize)]
// struct ApiResponse {
//     gmetadata: Vec<Metadata>,
// }

// /// Gallery metadata from EH API.
// /// See [EH API](https://ehwiki.org/wiki/API) for more information.
// #[derive(Deserialize, Debug)]
// pub struct Metadata {
//     gid: u32,
//     token: String,
//     archiver_key: String,
//     title: String,
//     title_jpn: String,
//     category: String,
//     thumb: String,
//     uploader: String,
//     posted: String,
//     filecount: String,
//     filesize: u32,
//     expunged: bool,
//     rating: String,
//     torrentcount: String,
//     tags: Vec<String>,
// }

// /// Gallery metadata returned by the querier.
// /// Notice `count` is always the total gallery count, which may not equal
// /// to the length of `data` if you did not chose the exhaustive query.
// #[derive(Debug)]
// pub struct MetaPack {
//     pub count: usize,
//     pub metadata: Vec<Metadata>,
// }

// /// A builder to construct and send a query.
// pub struct QueryBuilder<'a> {
//     querier: &'a Querier,
//     exhaustive: bool,
//     excluded_categories: Vec<String>,
//     terms: Vec<String>,
//     tags: HashMap<String, Vec<String>>,
// }

// impl<'a> QueryBuilder<'a> {
//     /// Build a blank query. The default value of `exhaustive` is `true`.
//     fn new(querier: &'a Querier) -> Self {
//         QueryBuilder {
//             querier,
//             exhaustive: true,
//             excluded_categories: Vec::new(),
//             terms: Vec::new(),
//             tags: HashMap::new(),
//         }
//     }

//     /// Whether to query detailed metadata for all found galleries or
//     /// only query the first page of the EH search result.
//     pub fn exhaustive(mut self, flag: bool) -> Self {
//         self.exhaustive = flag;
//         self
//     }

//     /// The category you do not want to search. Use the same category
//     /// names as EH except "Non-H" is changed to "Non H". Also notice
//     /// turning off all categories is the same as do not turn of any
//     /// category.
//     pub fn exclude_category(mut self, category: &str) -> Self {
//         self.excluded_categories.push(String::from(category));
//         self
//     }

//     /// The term you want to search. Each term will by surrounded by double-quotes.
//     /// For example, calling `term("123 456")` is the same as you type `"123 456"`
//     /// in the search bar of EH, which is different from typing `123 456`. See
//     /// [EH Gallery Searching](https://ehwiki.org/wiki/Gallery_Searching) for more
//     /// information.
//     pub fn term(mut self, term: &str) -> Self {
//         self.terms.push(String::from(term));
//         self
//     }

//     /// The tag you want to search. Calling `tag("a", "b c")` is the same as you
//     /// type `"a:b c$"` in the EH search bar. See
//     /// [EH Gallery Searching](https://ehwiki.org/wiki/Gallery_Searching)
//     /// for more information.
//     pub fn tag(mut self, tag: &str, term: &str) -> Self {
//         self.tags
//             .entry(String::from(tag))
//             .or_default()
//             .push(String::from(term));
//         self
//     }

//     /// Send the query.
//     pub async fn send(self) -> QuerierResult<MetaPack> {
//         self.querier.send(self).await
//     }
// }

// /// A EH Metadata Querier
// pub struct Querier {
//     client: Client,
//     ex: bool,
//     url: &'static str,
//     username: Option<String>,
//     password: Option<String>,
//     error_selector: Selector,
//     gallery_count_selector: Selector,
//     gallery_count_regex: Regex,
//     gallery_selector: Selector,
//     gallery_regex: Regex,
// }

// impl Querier {
//     /// Create an E-Hentai querier.
//     pub fn new() -> Self {
//         let client = reqwest::Client::builder()
//             .user_agent(USER_AGENT)
//             .cookie_store(true)
//             .timeout(Duration::new(TIMEOUT, 0))
//             .build()
//             .expect("Unable to initialize the request client.");
//         Querier {
//             client,
//             ex: false,
//             url: EH_URL,
//             username: None,
//             password: None,
//             error_selector: Selector::parse(ERROR_SELECTOR).unwrap(),
//             gallery_count_selector: Selector::parse(GALLERY_COUNT_SELECTOR).unwrap(),
//             gallery_count_regex: Regex::new(GALLERY_COUNT_REGEX).unwrap(),
//             gallery_selector: Selector::parse(GALLERY_SELECTOR).unwrap(),
//             gallery_regex: Regex::new(GALLERY_REGEX).unwrap(),
//         }
//     }

//     /// Create an EXHentai querier.
//     pub async fn new_ex(username: &str, password: &str) -> Self {
//         let client = reqwest::Client::builder()
//             .user_agent(USER_AGENT)
//             .cookie_store(true)
//             .timeout(Duration::new(TIMEOUT, 0))
//             .build()
//             .expect("Unable to initialize the request client.");

//         // Log in to the E-Hentai Forums to get access to EXHentai.
//         let login_params = [
//             ("UserName", username),
//             ("PassWord", password),
//             ("CookieDate", "1"),
//         ];
//         client
//             .post("https://forums.e-hentai.org/index.php?act=Login&CODE=01")
//             .form(&login_params)
//             .send()
//             .await
//             .expect("Unable to log in to ExHentai.");

//         // It seems you will always get the EXHentai homepage at the first request
//         // after you log in, so we just make a plain request to prime the client.
//         client
//             .get(EX_URL)
//             .send()
//             .await
//             .expect("Unable to log in to ExHentai.");

//         Querier {
//             client,
//             ex: true,
//             url: EX_URL,
//             username: Some(String::from(username)),
//             password: Some(String::from(password)),
//             error_selector: Selector::parse(ERROR_SELECTOR).unwrap(),
//             gallery_count_selector: Selector::parse(GALLERY_COUNT_SELECTOR).unwrap(),
//             gallery_count_regex: Regex::new(GALLERY_COUNT_REGEX).unwrap(),
//             gallery_selector: Selector::parse(GALLERY_SELECTOR).unwrap(),
//             gallery_regex: Regex::new(GALLERY_REGEX).unwrap(),
//         }
//     }

//     /// Query EH for galleries that you are interested in.
//     pub fn query(&self) -> QueryBuilder {
//         QueryBuilder::new(self)
//     }

//     /// Send the query.
//     async fn send(&self, query_builder: QueryBuilder<'_>) -> QuerierResult<MetaPack> {
//         // Computer the category.
//         let category = self.toggle_category(&query_builder.excluded_categories)?;

//         // Add search terms.
//         let mut search = String::new();
//         for term in query_builder.terms {
//             let segment = format!("+{:?}", term);
//             search.push_str(&segment);
//         }

//         // Add tags.
//         for (tag, values) in query_builder.tags {
//             for value in values {
//                 let segment = format!("\"+{}:{}$\"", tag, value);
//                 search.push_str(&segment);
//             }
//         }

//         // Get the first page of search results.
//         let page = self.request_page(0, &category, &search).await?;
//         let (count, mut galleries) = self.parse_page(&page)?;

//         // Get more results if `exhaustive` is set.
//         if query_builder.exhaustive {
//             let page_count = (count as f64 / GALLERIES_PER_PAGE as f64).ceil() as usize;
//             for cnt in 1..page_count {
//                 let page = self.request_page(cnt, &category, &search).await?;
//                 let (_, g) = self.parse_page(&page)?;
//                 galleries.extend(g);
//             }
//         }

//         // Query metadata.
//         let metadata = self.request_meta(&galleries).await?;

//         Ok(MetaPack { count, metadata })
//     }

//     /// Compute the category. See the document of `Category` for more information.
//     fn toggle_category(&self, categories: &[String]) -> QuerierResult<String> {
//         let mut category: Category = Default::default();
//         for cat in categories {
//             match cat.to_lowercase().as_str() {
//                 "doujinshi" => category |= Category::DOUJINSHI,
//                 "manga" => category |= Category::MANGA,
//                 "artist cg" => category |= Category::ARTIST_CG,
//                 "game cg" => category |= Category::GAME_CG,
//                 "western" => category |= Category::WESTERN,
//                 "non h" => category |= Category::NON_H,
//                 "image set" => category |= Category::IMAGE_SET,
//                 "cosplay" => category |= Category::COSPLAY,
//                 "asian porn" => category |= Category::ASIAN_PORN,
//                 "misc" => category |= Category::MISC,
//                 c => return Err(format!("Invalid category: {}.", c)),
//             }
//         }
//         Ok(String::from(category))
//     }

//     /// Request a research page.
//     async fn request_page(
//         &self,
//         page: usize,
//         category: &str,
//         search: &str,
//     ) -> Result<String, String> {
//         self.client
//             .get(self.url)
//             .query(&[("page", page)])
//             .query(&[("f_cats", category)])
//             .query(&[("f_search", search)])
//             .send()
//             .await
//             .map_err(|err| format!("{}", err))?
//             .text()
//             .await
//             .map_err(|err| format!("{}", err))
//     }

//     /// Extract the gallery count and galleries.
//     fn parse_page(&self, page: &str) -> QuerierResult<(usize, Vec<Gallery>)> {
//         let document = Html::parse_document(page);

//         // Check for the error message.
//         if let Some(elem) = document.select(&self.error_selector).next() {
//             return Err(elem.inner_html());
//         }

//         // Extract the gallery count.
//         let gallery_count_str =
//             if let Some(elem) = document.select(&self.gallery_count_selector).next() {
//                 elem.inner_html()
//             } else {
//                 // No hits found.
//                 String::from("0")
//             };
//         let gallery_count = self
//             .gallery_count_regex
//             .captures(&gallery_count_str)
//             .ok_or_else(|| String::from("Unable to extract the gallery count."))?
//             .get(0)
//             .ok_or_else(|| String::from("Unable to extract the gallery count."))?
//             .as_str()
//             .replace(",", "")
//             .parse::<usize>()
//             .map_err(|_| String::from("Unable to extract the gallery count."))?;

//         // Extract galleries.
//         let links = document.select(&self.gallery_selector).collect::<Vec<_>>();
//         let mut galleries = Vec::new();
//         for link in links {
//             let href = link
//                 .value()
//                 .attr("href")
//                 .ok_or("Unable to extract galleries.")?;
//             let caps = self
//                 .gallery_regex
//                 .captures(&href)
//                 .ok_or("Unable to extract galleries.")?;
//             let id = caps
//                 .get(1)
//                 .ok_or("Unable to extract galleries.")?
//                 .as_str()
//                 .parse()
//                 .map_err(|_| String::from("Unable to extract galleries."))?;
//             let token = caps
//                 .get(2)
//                 .ok_or("Unable to extract galleries.")?
//                 .as_str()
//                 .to_string();
//             galleries.push((id, token));
//         }

//         Ok((gallery_count, galleries))
//     }

//     /// Query metadata of galleries.
//     async fn request_meta(&self, galleries: &[Gallery]) -> QuerierResult<Vec<Metadata>> {
//         let mut metadata = Vec::new();
//         for chunk in galleries.chunks(GALLERIES_PER_PAGE) {
//             let params = ApiRequest::new(chunk.iter().cloned());
//             let meta = self
//                 .client
//                 .post(API_URL)
//                 .json(&params)
//                 .send()
//                 .await
//                 .map_err(|_| String::from("Unable to request gallery metadata."))?
//                 .json::<ApiResponse>()
//                 .await
//                 .map_err(|_| String::from("Unable to request gallery metadata."))?
//                 .gmetadata;
//             metadata.extend(meta);
//             time::delay_for(Duration::from_secs(DELAY)).await;
//         }
//         Ok(metadata)
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     async fn category() {
//         let querier = Querier::new();
//         let category = querier
//             .toggle_category(&[
//                 String::from("Doujinshi"),
//                 String::from("Manga"),
//                 String::from("Artist CG"),
//                 String::from("Game CG"),
//                 String::from("Western"),
//             ])
//             .unwrap();
//         assert_eq!(category, "542");
//         let category = querier
//             .toggle_category(&[
//                 String::from("Non H"),
//                 String::from("Image Set"),
//                 String::from("Cosplay"),
//                 String::from("Asian Porn"),
//                 String::from("Misc"),
//             ])
//             .unwrap();
//         assert_eq!(category, "481");
//     }

//     // To run this test, you need to provide an EXHentai account.
//     // #[tokio::test]
//     async fn query() {
//         let username = "";
//         let password = "";
//         let querier = Querier::new_ex(username, password).await;
//         let pack = querier
//             .query()
//             .exhaustive(true)
//             .exclude_category("Misc")
//             .term("密着エロ漫画家24時")
//             .term("pastchick3")
//             .tag("language", "chinese")
//             .send()
//             .await
//             .unwrap();
//         assert_eq!(pack.count, 1);
//         assert_eq!(pack.metadata[0].gid, 1053082);
//     }

//     #[tokio::test]
//     async fn not_exhaustive() {
//         let querier = Querier::new();
//         let pack = querier.query().exhaustive(false).send().await.unwrap();
//         assert!(pack.count > GALLERIES_PER_PAGE);
//         assert_eq!(pack.metadata.len(), GALLERIES_PER_PAGE);
//     }

//     #[tokio::test]
//     async fn error() {
//         let term = "vndiuhgiafsaidfhisfa:dgsgsdf";
//         let querier = Querier::new();
//         let result = querier.query().term(term).send().await;
//         assert!(result.unwrap_err().contains(term));
//     }

//     #[tokio::test]
//     async fn no_hits_found() {
//         let term = "vndiuhgiafsaidfhisfa";
//         let querier = Querier::new();
//         let pack = querier.query().term(term).send().await.unwrap();
//         assert_eq!(pack.count, 0);
//         assert_eq!(pack.metadata.len(), 0);
//     }
// }
