use log::{debug, info};
use serde_json::Value;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, Read, Write};
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use ureq::{Agent, AgentBuilder, Error, MiddlewareNext, Request, Response};

struct Progress {
    name: String,
    done: usize,
    total: usize,
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} => {}/{}", self.name, self.done, self.total)
    }
}

impl Progress {
    fn new(name: &str, total: usize) -> Self {
        let progress = Progress {
            name: String::from(name),
            done: 0,
            total,
        };
        progress.print_progress();
        progress
    }

    fn make_progress(&mut self) {
        self.done += 1;
        self.print_progress();
    }

    fn print_progress(&self) {
        if !self.name.is_empty() {
            print!("\r{}", self);
            io::stdout().flush().unwrap();
        }
    }

    fn finish(&self) {
        if !self.name.is_empty() {
            println!("");
            io::stdout().flush().unwrap();
        }
    }
}

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/99.0.4844.51 Safari/537.36 Edg/99.0.1150.39",
);

struct CrawlerRequest {
    id: usize,
    request: Request,
    retry: usize,
}

struct CrawlerResult {
    id: usize,
    result: Result<Vec<u8>, String>,
}

pub struct Crawler {
    agent: Agent,
    requests: Arc<Mutex<Vec<CrawlerRequest>>>,
    results: Arc<Mutex<Vec<CrawlerResult>>>,
    retry: usize,
    progress: Arc<Mutex<Progress>>,
}

impl Crawler {
    pub fn new(
        concurrency: usize,
        timeout: u64,
        headers: Vec<(&str, &str)>,
        cookies: Vec<(&str, &str)>,
        retry: usize,
    ) -> Self {
        info!("Build crawler - concurrency: {concurrency}, timeout: {timeout}, headers: {headers:?}, cookies: {cookies:?}, retry: {retry}");

        let requests = Arc::new(Mutex::new(Vec::new()));
        let results = Arc::new(Mutex::new(Vec::new()));
        let progress = Arc::new(Mutex::new(Progress::new("", 0)));

        for _ in 0..concurrency {
            let requests = requests.clone();
            let results = results.clone();
            let progress = progress.clone();
            thread::spawn(move || {
                let thread_id = thread::current().id();
                loop {
                    thread::sleep(Duration::from_millis(200));

                    let request = requests.lock().unwrap().pop();

                    if let Some(CrawlerRequest {
                        id,
                        request,
                        mut retry,
                    }) = request
                    {
                        let (buf, err) = match request.clone().call() {
                            Ok(resp) => {
                                let mut buf = Vec::new();
                                match resp.into_reader().read_to_end(&mut buf) {
                                    Ok(_) => (buf, String::new()),
                                    Err(err) => (Vec::new(), err.to_string()),
                                }
                                
                            }
                            Err(err) => (Vec::new(), err.to_string()),
                        };
                        if buf.is_empty() {
                            if retry == 0 {
                                debug!(
                                "Request fail ({thread_id:?})- id: {id}, request: {request:?}, error: {err:?}"
                            );
                                let result = CrawlerResult {
                                    id,
                                    result: Err(err),
                                };
                                results.lock().unwrap().push(result);
                            } else {
                                retry -= 1;
                                debug!("Retry request ({thread_id:?})- id: {id}, request: {request:?}, retry: {retry}, error: {err:?}");
                                let request = CrawlerRequest { id, request, retry };
                                requests.lock().unwrap().insert(0, request);
                            }
                        } else {
                            debug!("Request succeed ({thread_id:?}) - id: {id}, request: {request:?}");
                            let mut prog = progress.lock().unwrap();
                            prog.make_progress();
                            let result = CrawlerResult {
                                id,
                                result: Ok(buf),
                            };
                            results.lock().unwrap().push(result);
                        }
                    }
                }
            });
        }
        let headers: Vec<_> = headers
            .into_iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect();
        let add_headers = move |mut request: Request, next: MiddlewareNext| {
            for (name, value) in headers.clone() {
                request = request.set(&name, &value);
            }
            next.handle(request)
        };
        let cookies: Vec<_> = cookies
            .into_iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect();
        let add_cookies = move |request: Request, next: MiddlewareNext| {
            let mut cookie_str = String::new();
            for (name, value) in cookies.clone() {
                cookie_str.push_str(&format!("{name}={value};"));
            }
            next.handle(request.set("Cookie", &cookie_str))
        };
        let agent = AgentBuilder::new()
        .tls_connector(Arc::new(native_tls::TlsConnector::new().unwrap()))
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(timeout))
            .timeout_connect(Duration::from_secs(timeout))
            .middleware(add_headers)
            .middleware(add_cookies)
            .build();

        Crawler {
            agent,
            requests,
            results,
            retry,
            progress,
        }
    }

    pub fn get_text(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<String, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|rslt| rslt.map(|bytes|String::from_utf8(bytes).unwrap()))
            .collect()
    }

    pub fn get_json(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Value, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|rslt| rslt.map(|bytes| serde_json::from_slice(&bytes).unwrap()))
            .collect()
    }

    pub fn get_byte(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Vec<u8>, String>> {
        self.get(name, requests)
    }

    fn get(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Vec<u8>, String>> {
        let total = requests.len();
        let progress = Progress::new(name, total);
        *self.progress.lock().unwrap() = progress;

        let requests = requests
            .into_iter()
            .enumerate()
            .map(|(id, (url, queries))| {
                let mut request = self.agent.get(url);
                for (name, value) in queries {
                    request = request.query(name, value);
                }
                CrawlerRequest {
                    id,
                    request,
                    retry: self.retry,
                }
            })
            .collect();
        *self.requests.lock().unwrap() = requests;
        loop {
            thread::sleep(Duration::from_millis(200));
            let mut results = self.results.lock().unwrap();
            if results.len() == total {
                let mut results = mem::take(&mut *results);
                results.sort_unstable_by_key(|r| r.id);
                self.progress.lock().unwrap().finish();
                return results.into_iter().map(|r| r.result).collect();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Crawler, USER_AGENT};

    #[test]
    fn user_agent() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let mut results =
            crawler.get_json("", vec![("https://httpbin.org/user-agent", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let user_agent = json["user-agent"].as_str().unwrap();
        assert_eq!(user_agent, USER_AGENT);
    }

    #[test]
    fn header() {
        let crawler = Crawler::new(2, 15, vec![("K", "V")], Vec::new(), 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/headers", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let value = json["headers"]["K"].as_str().unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn cookie() {
        let crawler = Crawler::new(2, 15, Vec::new(), vec![("K", "V")], 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/cookies", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let value = json["cookies"]["K"].as_str().unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn query() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/get", vec![("K", "V")])]);
        let json = results.pop().unwrap().unwrap();
        let value = json["args"]["K"].as_str().unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn get_text() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let mut results = crawler.get_text("", vec![("https://httpbin.org/html", Vec::new())]);
        let text = results.pop().unwrap().unwrap();
        assert!(text.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn get_json() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/json", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let value = json["slideshow"]["title"].as_str().unwrap();
        assert_eq!(value, "Sample Slide Show");
    }

    #[test]
    fn get_byte() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let mut results = crawler.get_byte(
            "",
            vec![(
                "https://httpbin.org/base64/SFRUUEJJTiBpcyBhd2Vzb21l",
                Vec::new(),
            )],
        );
        let bytes = results.pop().unwrap().unwrap();
        assert_eq!(bytes, "HTTPBIN is awesome".as_bytes());
    }
}
