// mod querier;
// mod repository;

// use querier::Querier;
// use repository::Repository;

// #[macro_use]
// extern crate bitflags;

// #[macro_use]
// extern crate log;

use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "eh-manager")]
struct Opt {
    /// Username for E-Hentai Forums
    #[structopt(long)]
    username: Option<String>,

    /// Passward for E-Hentai Forums
    #[structopt(long)]
    password: Option<String>,

    /// Access ExHentai instead of E-Hentai
    #[structopt(long)]
    ex: bool,

    /// Require verbose logging
    #[structopt(long)]
    debug: bool,

    /// Host to run the eh-query server
    #[structopt(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to run the eh-query server
    #[structopt(long, default_value = "12345")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    // The logging level is determined in the following order:
    // If the command line flag `debug` is set, set the logging level to `debug`.
    // If the environment variable `RUST_LOG` is properly set, use that variable.
    // Otherwise set the logging level to `info`.
    if opt.debug {
        env::set_var("RUST_LOG", "debug");
    } else if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    };
    env_logger::init();

    // // If the command line flag `ex` is set and both the username and the password
    // // are provided, then we will query ExHentai. Otherwise, we will query E-Hentai.
    // let query = match (opt.username, opt.password, opt.ex) {
    //     (Some(name), Some(pw), true) => Query::new_ex(name, pw),
    //     _ => Query::new(),
    // };

    // // Bind the server to the provided address, default to `127.0.0.1:12345`.
    // // Then run the server forever until the shutdown command from the user.
    // let host: Ipv4Addr = opt.host.parse().expect("Invalid Host");
    // let addr = SocketAddrV4::new(host, opt.port);
    // server::serve(addr, query).await;
}
