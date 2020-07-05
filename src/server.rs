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
