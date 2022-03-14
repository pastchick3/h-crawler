mod crawler;
mod exhentai;
mod pixiv;

use clap::{Parser, Subcommand};
use crawler::Crawler;
use serde_derive::Deserialize;
use std::path::{Path, PathBuf};

const OUTPUT: &str = ".";
const TIMEOUT: u64 = 15;
const RETRY: usize = 1;
const CONCURRENCY: usize = 5;
const RELOAD: usize = 1;

#[derive(Parser)]
#[clap(version)]
pub struct Arguments {
    #[clap(long, parse(from_os_str))]
    pub config: Option<PathBuf>,

    #[clap(long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(long)]
    timeout: Option<u64>,

    #[clap(long)]
    retry: Option<usize>,

    #[clap(long)]
    concurrency: Option<usize>,

    #[clap(subcommand)]
    website: Option<Website>,
}

#[derive(Subcommand)]
enum Website {
    Exhentai {
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
    User { ids: Vec<String> },
    Artwork { ids: Vec<String> },
}

#[derive(Deserialize, Default, Debug)]
pub struct Config {
    output: Option<PathBuf>,
    timeout: Option<u64>,
    retry: Option<usize>,
    concurrency: Option<usize>,
    exhentai: Option<ExhentaiConfig>,
    pixiv: Option<PixivConfig>,
}

#[derive(Deserialize, Debug)]
struct ExhentaiConfig {
    reload: Option<usize>,
    ipb_member_id: Option<String>,
    ipb_pass_hash: Option<String>,
}

#[derive(Deserialize, Debug)]
struct PixivConfig {
    phpsessid: Option<String>,
}

pub fn run(arguments: Arguments, config: Config) {
    let output = arguments
        .output
        .and(config.output)
        .unwrap_or(Path::new(OUTPUT).to_path_buf());
    let timeout = arguments.timeout.and(config.timeout).unwrap_or(TIMEOUT);
    let retry = arguments.retry.and(config.retry).unwrap_or(RETRY);
    let concurrency = arguments
        .concurrency
        .and(config.concurrency)
        .unwrap_or(CONCURRENCY);

    match arguments.website {
        Some(Website::Exhentai {
            reload,
            ipb_member_id,
            ipb_pass_hash,
            galleries,
        }) => {
            let reload = reload
                .and(
                    config
                        .exhentai
                        .as_ref()
                        .map(|eh| eh.reload.as_ref())
                        .flatten(),
                )
                .cloned()
                .unwrap_or(RELOAD);
            let ipb_member_id = ipb_member_id
                .or(config
                    .exhentai
                    .as_ref()
                    .map(|eh| eh.ipb_member_id.clone())
                    .flatten())
                .expect("`ipb_member_id` is not defined")
                .clone();
            let ipb_pass_hash = ipb_pass_hash
                .or(config
                    .exhentai
                    .as_ref()
                    .map(|eh| eh.ipb_pass_hash.clone())
                    .flatten())
                .expect("`ipb_pass_hash` is not defined")
                .clone();
            let cookies = vec![
                ("ipb_member_id", ipb_member_id.as_str()),
                ("ipb_pass_hash", ipb_pass_hash.as_str()),
            ];
            let crawler = Crawler::new(concurrency, timeout, Vec::new(), cookies, retry);
            exhentai::crawl(crawler, output, reload, galleries);
        }
        Some(Website::Pixiv { phpsessid, target }) => {
            let phpsessid = phpsessid
                .or(config
                    .pixiv
                    .as_ref()
                    .map(|px| px.phpsessid.clone())
                    .flatten())
                .expect("`phpsessid` is not defined")
                .clone();
            let crawler = Crawler::new(
                concurrency,
                timeout,
                vec![("Referer", "https://www.pixiv.net/")],
                vec![("PHPSESSID", &phpsessid)],
                retry,
            );
            match target {
                Some(PixivTarget::User { ids }) => pixiv::crawl_users(crawler, output, ids),
                Some(PixivTarget::Artwork { ids }) => {
                    pixiv::crawl_artworks(&crawler, output, String::from("Unnamed"), ids)
                }
                None => (),
            }
        }
        None => (),
    }
}
