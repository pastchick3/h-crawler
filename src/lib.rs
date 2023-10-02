mod crawler;
mod exhentai;
mod fanbox;
mod pixiv;

use clap::{Parser, Subcommand};
use crawler::Crawler;
use log::info;
use serde_derive::Deserialize;
use std::path::{Path, PathBuf};

const CONCURRENCY: usize = 8;
const TIMEOUT: u64 = 30;
const RETRY: usize = 1;
const OUTPUT: &str = ".";
const RELOAD: usize = 1;

#[derive(Parser, Debug)]
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

#[derive(Subcommand, Debug)]
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
    Fanbox {
        #[clap(long)]
        fanboxsessid: Option<String>,

        posts: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum PixivTarget {
    User { users: Vec<String> },
    Illust { illusts: Vec<String> },
}

#[derive(Deserialize, Default, Debug)]
pub struct Config {
    concurrency: Option<usize>,
    timeout: Option<u64>,
    retry: Option<usize>,
    output: Option<PathBuf>,
    exhentai: Option<ExhentaiConfig>,
    pixiv: Option<PixivConfig>,
    fanbox: Option<FanboxConfig>,
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

#[derive(Deserialize, Debug)]
struct FanboxConfig {
    fanboxsessid: Option<String>,
}

pub fn run(arguments: Arguments, config: Config) {
    info!("{arguments:?}");
    info!("{config:?}");

    let concurrency = arguments
        .concurrency
        .or(config.concurrency)
        .unwrap_or(CONCURRENCY);
    let timeout = arguments.timeout.or(config.timeout).unwrap_or(TIMEOUT);
    let retry = arguments.retry.or(config.retry).unwrap_or(RETRY);
    let output = arguments
        .output
        .or(config.output)
        .unwrap_or_else(|| Path::new(OUTPUT).to_path_buf());
    match arguments.website {
        Some(Website::Exhentai {
            reload,
            ipb_member_id,
            ipb_pass_hash,
            galleries,
        }) => {
            let reload = reload
                .or_else(|| config.exhentai.as_ref().and_then(|eh| eh.reload))
                .unwrap_or(RELOAD);
            let ipb_member_id = ipb_member_id
                .or_else(|| {
                    config
                        .exhentai
                        .as_ref()
                        .and_then(|eh| eh.ipb_member_id.clone())
                })
                .expect("`ipb_member_id` is not defined");
            let ipb_pass_hash = ipb_pass_hash
                .or_else(|| {
                    config
                        .exhentai
                        .as_ref()
                        .and_then(|eh| eh.ipb_pass_hash.clone())
                })
                .expect("`ipb_pass_hash` is not defined");
            let cookies = vec![
                ("ipb_member_id", ipb_member_id.as_str()),
                ("ipb_pass_hash", ipb_pass_hash.as_str()),
            ];
            let crawler = Crawler::new(concurrency, timeout, Vec::new(), cookies, retry);
            exhentai::crawl_galleries(&crawler, output, reload, galleries);
        }
        Some(Website::Pixiv { phpsessid, target }) => {
            let phpsessid = phpsessid
                .or_else(|| config.pixiv.as_ref().and_then(|px| px.phpsessid.clone()))
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
        Some(Website::Fanbox {
            fanboxsessid,
            posts,
        }) => {
            let fanboxsessid = fanboxsessid
                .or(config.fanbox.and_then(|fb| fb.fanboxsessid))
                .expect("`FANBOXSESSID` is not defined");
            let crawler = Crawler::new(
                concurrency,
                timeout,
                vec![("Origin", "https://www.fanbox.cc")],
                vec![("FANBOXSESSID", &fanboxsessid)],
                retry,
            );
            fanbox::crawl_posts(&crawler, output, posts);
        }
        None => {}
    }
}
