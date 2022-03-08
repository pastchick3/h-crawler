// mod crawler;

// use crawler::Crawler;
// use serde_derive::Deserialize;
use std::path::PathBuf;

const EH_CREDENTIAL: &str = "./eh-credential";


use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(version)]
struct Args {
    /// Optional name to operate on
    name: Option<String>,

    /// Sets a custom config file
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[clap(short, long, parse(from_occurrences))]
    debug: usize,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Test {
        /// lists test values
        #[clap(short, long)]
        list: bool,
    },
}


// #[derive(Deserialize)]
// pub struct Credential {
//     ipb_member_id: String,
//     ipb_pass_hash: String,
// }

fn main() {
    // let credential_str = fs::read_to_string(EH_CREDENTIAL).unwrap();
    // let credential: Credential = toml::from_str(&credential_str).unwrap();

    let args = Args::parse();

}
