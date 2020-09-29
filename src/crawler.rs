use log::{debug, error, info, warn};
use std::fmt::Display;
use std::time::Duration;
use reqwest::{Client, Error};
use scraper::{Html, Selector};
use tokio::time;
use reqwest::header;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);



const EX_URL: &str = "https://exhentai.org/";
const TIMEOUT: u64 = 30; // request timeout (in second)
const DELAY: u64 = 2; // delay after each request (in second)
const ERROR_SELECTOR: &str = "#iw p";
const GALLERY_COUNT_SELECTOR: &str = "div.ido div:nth-child(2) p.ip";
const GALLERY_SELECTOR: &str = "div.ido table.itg.gltc td.gl3c.glname a";

pub struct Crawler {
    client: Client,
}

impl Crawler {
    pub fn new(username: &str, password: &str) -> Result<Self, Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::COOKIE, header::HeaderValue::from_static(COOKIE));
        

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT)
            .cookie_store(true)
            .timeout(Duration::new(TIMEOUT, 0))
            .build()?;


        Ok(Crawler {
            client
        })
    }


    pub async fn crawl(
        &self,
        artist: &str,
        title: &str,
        url: &str,
        start: Option<u16>,
        end: Option<u16>,
    ) -> Result<(), Box<dyn Display>>{
        // Request the first index.
        let page = self.client
            .get(url)
            .send()
            .await.unwrap()
            .text()
            .await.unwrap();
        let (mut images, num) = self.extract_index_pages(&page);

        // Request other index pages.
        // 
        // time::delay_for(Duration::from_secs(DELAY)).await;
        for i in 1..num {
            let page = self.client
            .get(url)
            .query(&[("p", i)])
            .send()
            .await.unwrap()
            .text()
            .await.unwrap();
            let imgs = self.extract_image_links(&page);
            images.extend(imgs);
        }
        
        // Request for img pages.
        let mut links = Vec::new();
        for url in images {
            let link = self.get_link(&url).await;
            links.push(link);
        }
        
        let path = PathBuf::from(format!("./[{}] {}", artist, title));
        fs::create_dir(&path).unwrap();
        // Download images.
        for (i, link) in links.iter().enumerate() {
            self.download(&link, &path, i).await;
        }


        Ok(())
    }

    fn extract_index_pages(&self, page: &str) -> (Vec<String>, u16) {
        let images = self.extract_image_links(page);
        
        let document = Html::parse_document(page);

        // Extract max page.
        let selector = Selector::parse("#asm + div td:nth-last-child(2) > a").unwrap();
        let num: u16 = document.select(&selector).next().unwrap().inner_html().parse().unwrap();
        
        return (images, num);

    }

    fn extract_image_links(&self, page: &str) -> Vec<String> {
        let document = Html::parse_document(page);

        // Extract images in this page.
        let selector = Selector::parse("#gdt a").unwrap();
        document.select(&selector)
            .map(|elem| elem.value().attr("href").unwrap().to_string())
            .collect()
    }

    async fn get_link(&self, url: &str) -> String {
        let page = self.client
            .get(url)
            .send()
            .await.unwrap()
            .text()
            .await.unwrap();

        let document = Html::parse_document(&page);

        let selector = Selector::parse("#img").unwrap();
        document.select(&selector).next().unwrap().value().attr("src").unwrap().to_string()
    }

    async fn download(&self, link: &str, path: &Path, i: usize) {
        let bytes = self.client
            .get(link)
            .send()
            .await.unwrap()
            .bytes()
            .await.unwrap();
        let name = format!("{}.jpg", i);
        let mut file = fs::File::create(path.join(name)).unwrap();
        file.write_all(&bytes).unwrap();
    }
}





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
