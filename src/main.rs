#[macro_use]
extern crate lazy_static;

mod crawler;

use crawler::Crawler;
use serde_derive::Deserialize;
use std::fs;
use structopt::StructOpt;

const EH_CREDENTIAL: &str = "./eh-credential";
const EH_BASE_URL: &str = "https://exhentai.org/g/";

#[derive(Deserialize)]
pub struct Credential {
    ipb_member_id: String,
    ipb_pass_hash: String,
}

#[derive(StructOpt)]
#[structopt(name = "eh-crawler")]
struct Opt {
    galleries: Vec<String>,
}

#[tokio::main]
async fn main() {
    let credential_str = fs::read_to_string(EH_CREDENTIAL).unwrap();
    let credential: Credential = toml::from_str(&credential_str).unwrap();

    let opt = Opt::from_args();
    let mut galleries = Vec::new();
    for gallery in opt.galleries {
        let parts: Vec<_> = gallery.split('/').collect();
        if parts.len() != 3 {
            panic!("Invalid gallery `{}`.", gallery);
        }
        let url = format!("{}/{}/{}/", EH_BASE_URL, parts[0], parts[1]);
        let range = match parts[2] {
            "" => (None, None),
            range => {
                let range: Vec<_> = range.split('-').collect();
                if range.len() != 2 {
                    panic!("Invalid range `{}`.", gallery);
                }
                let start = range[0].parse().unwrap();
                let end = range[1].parse().unwrap();
                (Some(start), Some(end))
            }
        };
        galleries.push((url, range));
    }

    let mut crawler = Crawler::new(credential);
    crawler.crawl(galleries).await;
}
