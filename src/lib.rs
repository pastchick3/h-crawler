mod crawler;
mod pixiv;
mod exhentai;

use clap::{Parser, Subcommand};
use serde_derive::Deserialize;
use std::path::{Path, PathBuf};
use crawler::Crawler;

const OUTPUT: &str = ".";
const VERBOSE: bool = false;
const TIMEOUT: usize = 15;
const RETRY: usize = 1;
const CONCURRENCY: usize = 5;
const RELOAD: usize = 1;

#[derive(Parser)]
#[clap(version)]
pub struct Arguments {
    #[clap(long, parse(from_os_str))]
    config: Option<PathBuf>,

    #[clap(long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(long)]
    timeout: Option<usize>,

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
        #[clap(subcommand)]
        target: Option<PixivTarget>,
    },
}

#[derive(Subcommand)]
enum PixivTarget {
    User { ids: Vec<String> },
    Artwork { ids: Vec<String> },
}

#[derive(Deserialize, Default)]
pub struct Config {
    output: Option<PathBuf>,
    timeout: Option<usize>,
    retry: Option<usize>,
    concurrency: Option<usize>,
    exhentai: Option<ExhentaiConfig>,
}

#[derive(Deserialize)]
struct ExhentaiConfig {
    reload: Option<usize>,
    ipb_member_id: Option<String>,
    ipb_pass_hash: Option<String>,
}

pub fn run(arguments: Arguments, config: Config) {
    let output= arguments
            .output
            .and(config.output)
            .unwrap_or(Path::new(OUTPUT).to_path_buf());
        let timeout= arguments
            .timeout
            .and(config.timeout)
            .unwrap_or(TIMEOUT);
        let retry= arguments.retry.and(config.retry).unwrap_or(RETRY);
        let concurrency=  arguments
            .concurrency
            .and(config.concurrency)
            .unwrap_or(CONCURRENCY);
    let crawler = Crawler::new(output, timeout, retry, concurrency);


    match arguments.website {
        Some(Website::Exhentai {
            reload,
            ipb_member_id,
            ipb_pass_hash,
            galleries,
        }) => {
                let reload = reload
                    .and(config.exhentai.map(|eh| eh.reload).flatten())
                    .unwrap_or(RELOAD);
                let ipb_member_id = ipb_member_id.and(config.exhentai.map(|eh| eh.ipb_member_id).flatten())
                .expect("`ipb_member_id` is not defined");
                let ipb_pass_hash = ipb_pass_hash.and(config.exhentai.map(|eh| eh.ipb_pass_hash).flatten())
                .expect("`ipb_pass_hash` is not defined");
            exhentai::crawl(crawler, reload, ipb_member_id, ipb_pass_hash, galleries);
        }
        Some(Website::Pixiv { target }) => match target {
            Some(PixivTarget::User { ids }) => pixiv::crawl_user(crawler, ids),
            Some(PixivTarget::Artwork { ids }) => pixiv::crawl_artwork(crawler, ids),
            None => (),
        },
        None => (),
    }
}