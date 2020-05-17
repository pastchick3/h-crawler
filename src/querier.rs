use std::collections::HashMap;
use std::time::Duration;

use reqwest::Client;
use scraper::Html;

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);
const EH_URL: &str = "https://e-hentai.org/";
const EX_URL: &str = "https://exhentai.org/";
const TIMEOUT: u64 = 30;

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

// {
//     "gid": 618395,
//     "token": "0439fa3666",
//     "archiver_key": "403565--d887c6dfe8aae79ed0071551aa1bafeb4a5ee361",
//     "title": "(Kouroumu 8) [Handful☆Happiness! (Fuyuki Nanahara)] TOUHOU GUNMANIA A2 (Touhou Project)",
//     "title_jpn": "(紅楼夢8) [Handful☆Happiness! (七原冬雪)] TOUHOU GUNMANIA A2 (東方Project)",
//     "category": "Non-H",
//     "thumb": "https://ehgt.org/14/63/1463dfbc16847c9ebef92c46a90e21ca881b2a12-1729712-4271-6032-jpg_l.jpg",
//     "uploader": "avexotsukaai",
//     "posted": "1376143500",
//     "filecount": "20",
//     "filesize": 51210504,
//     "expunged": false,
//     "rating": "4.43",
//     "torrentcount": "0",
//     "tags": [
//         "parody:touhou project",
//         "group:handful happiness",
//         "artist:nanahara fuyuki",
//         "full color",
//         "artbook"
//     ]
// }
pub struct EHResponse {
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

pub struct QueryResult {
    count: u64,
    galleries: Vec<EHResponse>,
}

pub struct Querier {
    client: Client,
    username: Option<String>,
    password: Option<String>,
    ex: bool,
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
            username: None,
            password: None,
            ex: false,
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
            username: Some(username),
            password: Some(password),
            ex: true,
        }
    }

    /// {
    ///     "Category": Vec<String>,
    ///     "Search": Vec<String>,
    ///     tag: Vec<String>,
    /// }
    pub async fn query(
        &self,
        exclusive: bool,
        params: HashMap<String, Vec<String>>,
    ) -> Result<QueryResult, String> {
        let mut category = String::from("0");
        let mut query_string = String::new();
        for (key, value) in params.iter() {
            match key.to_lowercase().as_str() {
                "category" => {
                    category = self.toggle_category(value)?;
                }
                "search" => {
                    for term in value.iter() {
                        let segment = format!("+{:?}", term);
                        query_string.push_str(&segment);
                    }
                }
                tag => {
                    for term in value.iter() {
                        let segment = format!("+{}:{:?}$", tag, term);
                        query_string.push_str(&segment);
                    }
                }
            }
        }
        let page = self
            .client
            .get(EX_URL)
            .query(&[("page", 0u8)])
            .query(&[("f_cats", &category)])
            .query(&[("f_search", &query_string)])
            .send()
            .await
            //.log_err()
            .map_err(|_| String::from("Unable to load the first page."))?
            .text()
            .await
            .unwrap();
        let (page_count, mut galleries) = self.parse_page(&page);
        for pg in 1..page_count {
            let (_, g) = self.parse_page(&page);
            self.client
                .get(EX_URL)
                .query(&[("page", pg)])
                .query(&[("f_cats", &category)])
                .query(&[("f_search", &query_string)])
                .send()
                .await
                //.log_err()
                .map_err(|_| String::from("Unable to load the first page."))?;
            galleries.extend(g);
        }

        self.query_api(galleries)
    }

    fn parse_page(&self, page: &str) -> (u64, Vec<(u64, u64)>) {
        let document = Html::parse_document(page);
        todo!()
    }

    fn query_api(&self, galleries: Vec<(u64, u64)>) -> Result<QueryResult, String> {
        todo!()
    }

    // function toggle_category(b) {
    //     // 每关一个就 | 对应的值
    //     var a = document.getElementById("f_cats"); // init 0
    //     var c = document.getElementById("cat_" + b);
    //     if (a.getAttribute("disabled")) {
    //         a.removeAttribute("disabled")
    //     }
    //     if (c.getAttribute("data-disabled")) {
    //         c.removeAttribute("data-disabled");
    //         a.value = parseInt(a.value) & (1023 ^ b)
    //     } else {
    //         c.setAttribute("data-disabled", 1);
    //         a.value = parseInt(a.value) | b
    //     }
    // }
    fn toggle_category(&self, categories: &[String]) -> Result<String, String> {
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
                _ => return Err(String::from(cat)),
            }
        }
        Ok(String::from(!category))
    }
}
