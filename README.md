# eh-manager

Manage your online and offline EH resources with ease.

## 存储
    - query
    - 存储入口
    - 存储界面
## 智能导入
## 智能浏览

https://ehwiki.org/wiki/Gallery_Searching

title/tag
"exact match"
female:piercing$

Searches may only be performed once every 3 seconds.

http://ehwiki.org/wiki/API

25 entries per request, 4-5 sequential requests usually okay before having to wait for ~5 seconds

structopt = "0.3"
tokio = { version = "0.2", features = ["macros", "rt-threaded"] }
reqwest = { version = "0.10", features = ["cookies"] }
warp = "0.2"
scraper = "0.12"
log = "0.4"
env_logger = "0.7"


/// {
///     "Category": Vec<String>,
///     "Search": Vec<String>,
///     tag: Vec<String>,
/// }

// function toggle_category(b) {
    //     // 每关一个就 | 对应的值
    //     var a = document.getElementById("f_cats"); // init 0
    //     var c = document.getElementById("cat_" + b);
    //     if (a.getAttribute("disabled")) {
    //         a.removeAttribute("disabled")
    //     }
    //     if (c.getAttribute("data-disabled")) {
    //         c.removeAttribute("data-disabled");
    //         a.value = parseInt(a.value) & (1023 ^ b)
    //     } else {
    //         c.setAttribute("data-disabled", 1);
    //         a.value = parseInt(a.value) | b
    //     }
    // }
    
## main
mod query;
mod server;

#[macro_use]
extern crate log;

use std::env;
use std::net::{Ipv4Addr, SocketAddrV4};

use structopt::StructOpt;

use query::Query;

#[derive(StructOpt, Debug)]
#[structopt(name = "eh-query")]
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

    // If the command line flag `ex` is set and both the username and the password
    // are provided, then we will query ExHentai. Otherwise, we will query E-Hentai.
    let query = match (opt.username, opt.password, opt.ex) {
        (Some(name), Some(pw), true) => Query::new_ex(name, pw),
        _ => Query::new(),
    };

    // Bind the server to the provided address, default to `127.0.0.1:12345`.
    // Then run the server forever until the shutdown command from the user.
    let host: Ipv4Addr = opt.host.parse().expect("Invalid Host");
    let addr = SocketAddrV4::new(host, opt.port);
    server::serve(addr, query).await;
}


## server

use std::collections::HashMap;
use std::net::SocketAddrV4;
use std::time::Duration;

use tokio::time;
use warp::Filter;

use crate::query::Query;

/// A global flag used to programmatically stop the server.
static mut RUNNING: bool = true;

/// Run the eh-query server at `addr` using the given `query`.
/// This server is configured with two routes:
///
/// (1) `GET /shutdown`: stop the server. Notice when tested in Windows,
/// it may take 30 seconds to fully stop the server. Or you can initiate
/// an arbitrary request to help the server stop immediately.
///
/// (2) `POST /query`: query the E-Hentai/ExHentai. The request body should
/// be a JSON which will be fed into `query`. The response body will also be
/// a JSON returned by `query`. Please refer to the document of `query` for
/// more details.
pub async fn serve(addr: SocketAddrV4, query: Query) {
    // `GET /shutdown`
    let shutdown_filter = warp::path("shutdown").and(warp::get()).map(|| {
        unsafe {
            RUNNING = false;
        }
        warp::reply()
    });

    // `POST /query`
    let query_filter = warp::path("query")
        .and(warp::post())
        .and(warp::body::json())
        .map(move |param: HashMap<String, String>| {
            let result = query.query(param);
            warp::reply::json(&result)
        });

    // Run the server.
    let filters = warp::any().and(shutdown_filter.or(query_filter));
    let (_, server) = warp::serve(filters).bind_with_graceful_shutdown(addr, async {
        while unsafe { RUNNING } {
            time::delay_for(Duration::from_millis(500)).await;
        }
    });
    info!("eh-query server is running at {:?}.", addr);
    tokio::task::spawn(server)
        .await
        .expect("Fail to execute the eh-query server.");
}
