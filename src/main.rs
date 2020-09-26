mod crawler;
mod database;

use env_logger::Env;
use std::path::PathBuf;
use structopt::StructOpt;

use database::Database;

#[derive(StructOpt)]
#[structopt(name = "eh-manager")]
struct Opt {
    username: String,

    password: String,

    #[structopt(long, parse(from_os_str), default_value = ".")]
    resource: PathBuf,

    #[structopt(long)]
    debug: bool,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    let env = if opt.debug {
        Env::default().default_filter_or("debug")
    } else {
        Env::default().default_filter_or("warn")
    };
    env_logger::from_env(env).init();
    let database = Database::new(&opt.username, &opt.password, &opt.resource);

    repl(&database);
}

fn repl(database: &Database) {}
