mod crawler;
mod ehentai;
mod pixiv;

use clap::{Parser, Subcommand};
use crawler::Crawler;
use serde_derive::Deserialize;
use std::path::{Path, PathBuf};

const CONCURRENCY: usize = 5;
const TIMEOUT: u64 = 60;
const RETRY: usize = 1;
const OUTPUT: &str = ".";
const X: bool = false;
const RELOAD: usize = 1;

#[derive(Parser)]
#[clap(version)]
pub struct Arguments {
    #[clap(long, parse(from_os_str))]
    pub config: Option<PathBuf>,

    #[clap(long)]
    concurrency: Option<usize>,

    #[clap(long)]
    timeout: Option<u64>,

    #[clap(long)]
    retry: Option<usize>,

    #[clap(long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(subcommand)]
    website: Option<Website>,
}

#[derive(Subcommand)]
enum Website {
    Ehentai {
        #[clap(long)]
        x: bool,

        #[clap(long)]
        reload: Option<usize>,

        #[clap(long)]
        ipb_member_id: Option<String>,

        #[clap(long)]
        ipb_pass_hash: Option<String>,

        galleries: Vec<String>,
    },
    Pixiv {
        #[clap(long)]
        phpsessid: Option<String>,

        #[clap(subcommand)]
        target: Option<PixivTarget>,
    },
}

#[derive(Subcommand)]
enum PixivTarget {
    User { users: Vec<String> },
    Illust { illusts: Vec<String> },
}

#[derive(Deserialize, Default)]
pub struct Config {
    concurrency: Option<usize>,
    timeout: Option<u64>,
    retry: Option<usize>,
    output: Option<PathBuf>,
    ehentai: Option<EhentaiConfig>,
    pixiv: Option<PixivConfig>,
}

#[derive(Deserialize)]
struct EhentaiConfig {
    x: Option<bool>,
    reload: Option<usize>,
    ipb_member_id: Option<String>,
    ipb_pass_hash: Option<String>,
}

#[derive(Deserialize)]
struct PixivConfig {
    phpsessid: Option<String>,
}

pub fn run(arguments: Arguments, config: Config) {
    let concurrency = arguments
        .concurrency
        .or(config.concurrency)
        .unwrap_or(CONCURRENCY);
    let timeout = arguments.timeout.or(config.timeout).unwrap_or(TIMEOUT);
    let retry = arguments.retry.or(config.retry).unwrap_or(RETRY);
    let output = arguments
        .output
        .or(config.output)
        .unwrap_or(Path::new(OUTPUT).to_path_buf());
    match arguments.website {
        Some(Website::Ehentai {
            x,
            reload,
            ipb_member_id,
            ipb_pass_hash,
            galleries,
        }) => {
            let x = x | config
                .ehentai
                .as_ref()
                .map(|eh| eh.x)
                .flatten()
                .unwrap_or(X);
            let reload = reload
                .or(config.ehentai.as_ref().map(|eh| eh.reload).flatten())
                .unwrap_or(RELOAD);
            let ipb_member_id = ipb_member_id
                .or(config
                    .ehentai
                    .as_ref()
                    .map(|eh| eh.ipb_member_id.clone())
                    .flatten())
                .expect("`ipb_member_id` is not defined");
            let ipb_pass_hash = ipb_pass_hash
                .or(config
                    .ehentai
                    .as_ref()
                    .map(|eh| eh.ipb_pass_hash.clone())
                    .flatten())
                .expect("`ipb_pass_hash` is not defined");
            let cookies = vec![
                ("ipb_member_id", ipb_member_id.as_str()),
                ("ipb_pass_hash", ipb_pass_hash.as_str()),
            ];
            let crawler = Crawler::new(concurrency, timeout, Vec::new(), cookies, retry);
            ehentai::crawl_galleries(&crawler, output, x, reload, galleries);
        }
        Some(Website::Pixiv { phpsessid, target }) => {
            let phpsessid = phpsessid
                .or(config
                    .pixiv
                    .as_ref()
                    .map(|px| px.phpsessid.clone())
                    .flatten())
                .expect("`phpsessid` is not defined");
            let crawler = Crawler::new(
                concurrency,
                timeout,
                vec![("Referer", "https://www.pixiv.net/")],
                vec![("PHPSESSID", &phpsessid)],
                retry,
            );
            match target {
                Some(PixivTarget::User { users }) => pixiv::crawl_users(&crawler, output, users),
                Some(PixivTarget::Illust { illusts }) => {
                    pixiv::crawl_illusts(&crawler, output, illusts)
                }
                None => (),
            }
        }
        None => {}
    }
}
