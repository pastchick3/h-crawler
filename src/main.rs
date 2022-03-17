use clap::Parser;
use h_crawler::{self, Arguments, Config};
use std::fs;
use std::path::Path;

const CONFIG: &str = "./h-config.toml";

fn main() {
    env_logger::init();

    let arguments = Arguments::parse();
    let config_path = arguments
        .config
        .clone()
        .unwrap_or_else(|| Path::new(CONFIG).to_path_buf());
    let config = match fs::read_to_string(config_path) {
        Ok(config) => toml::from_str(&config).unwrap(),
        Err(_) => Config::default(),
    };
    h_crawler::run(arguments, config);
}
