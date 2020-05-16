use std::collections::HashMap;
use std::time::Duration;

use reqwest::Client;

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
    ///     "Category": [String],
    ///     "Search": [String],
    ///     tag: [String],
    /// }
    pub async fn query(&self, params: HashMap<String, String>) -> HashMap<String, String> {
        // toggle_category
        HashMap::new()
        //
    }

    fn toggle_category(&self, categories: &[String])-> Result<String, String> {
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
                _ => {return Err(String::from(cat))}
            }
        }
        Ok(String::from(category))
    }
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
