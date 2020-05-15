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

    pub async fn query(&self, params: HashMap<String, String>) -> HashMap<String, String> {
        HashMap::new()
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
