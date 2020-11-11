mod crawler;

use crawler::Crawler;
use serde_derive::Deserialize;
use std::fs;
use structopt::StructOpt;

const EH_CREDENTIAL: &str = "./eh-credential";

#[derive(Deserialize)]
pub struct Credential {
    ipb_member_id: String,
    ipb_pass_hash: String,
}

#[derive(StructOpt)]
#[structopt(name = "eh-crawler")]
struct Opt {
    url: String,
    range: Option<String>,
}

#[tokio::main]
async fn main() {
    let credential_str = fs::read_to_string(EH_CREDENTIAL).unwrap();
    let credential: Credential = toml::from_str(&credential_str).unwrap();

    let opt = Opt::from_args();
    let (start, end) = match opt.range {
        Some(range) => {
            let range: Vec<_> = range.split('-').collect();
            if range.len() != 2 {
                panic!("Invalid range.");
            }
            let start = range[0].parse().unwrap();
            let end = range[1].parse().unwrap();
            (Some(start), Some(end))
        }
        None => (None, None),
    };

    let crawler = Crawler::new(credential);
    crawler.crawl(&opt.url, start, end).await;
}
