use log::{debug, info};
use reqwest::blocking::{Client, Request};
use reqwest::header::{HeaderMap, HeaderName};
use serde_json::Value;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, Write};
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/99.0.4844.51 Safari/537.36 Edg/99.0.1150.39",
);

struct Progress {
    name: String,
    done: usize,
    total: usize,
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
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
            println!();
            io::stdout().flush().unwrap();
        }
    }
}

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
    client: Arc<Mutex<Client>>,
    requests: Arc<Mutex<Vec<CrawlerRequest>>>,
    results: Arc<Mutex<Vec<CrawlerResult>>>,
    progress: Arc<Mutex<Progress>>,
    retry: usize,
}

impl Crawler {
    pub fn new(
        concurrency: usize,
        timeout: u64,
        headers: Vec<(&str, &str)>,
        cookies: Vec<(&str, &str)>,
        retry: usize,
    ) -> Self {
        // Initialize the HTTP client.
        let mut default_headers = HeaderMap::new();
        for (name, value) in headers {
            let name = HeaderName::from_bytes(name.as_bytes()).unwrap();
            default_headers.append(name, value.parse().unwrap());
        }
        let mut cookie_str = String::new();
        for (name, value) in cookies {
            cookie_str.push_str(&format!("{name}={value};"));
        }
        default_headers.append("Cookie", cookie_str.parse().unwrap());
        let client = Arc::new(Mutex::new(
            Client::builder()
                .user_agent(USER_AGENT)
                .timeout(Some(Duration::from_secs(timeout)))
                .default_headers(default_headers)
                .build()
                .unwrap(),
        ));

        // Spawn worker threads.
        let requests = Arc::new(Mutex::new(Vec::new()));
        let results = Arc::new(Mutex::new(Vec::new()));
        let progress = Arc::new(Mutex::new(Progress::new("", 0)));
        for c in 0..concurrency {
            let client = client.clone();
            let requests = requests.clone();
            let results = results.clone();
            let progress = progress.clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_millis(1000 / concurrency as u64));

                // Check for waiting requests.
                let request = requests.lock().unwrap().pop();
                if let Some(CrawlerRequest { id, request, retry }) = request {
                    debug!("Request {id} - Start in Thread {c}: {request:?}");

                    // Execute the request.
                    let result = client.lock().unwrap().execute(request.try_clone().unwrap());
                    let result = match result {
                        Ok(resp) if resp.status().is_success() => match resp.bytes() {
                            Ok(bytes) => Ok(bytes.to_vec()),
                            Err(err) => Err(err.to_string()),
                        },
                        Ok(resp) => Err(resp.status().to_string()),
                        Err(err) => Err(err.to_string()),
                    };

                    // Handle the response.
                    match result {
                        result @ Ok(_) => {
                            debug!("Request {id} - Succeed in Thread {c}");

                            progress.lock().unwrap().make_progress();
                            results.lock().unwrap().push(CrawlerResult { id, result });
                        }
                        Err(err) if retry == 0 => {
                            debug!("Request {id} - Fail in Thread {c}: {err}");

                            results.lock().unwrap().push(CrawlerResult {
                                id,
                                result: Err(err),
                            });
                        }
                        Err(err) => {
                            debug!("Request {id} - Retry in Thread {c}: {err}");

                            requests.lock().unwrap().insert(
                                0,
                                CrawlerRequest {
                                    id,
                                    request,
                                    retry: retry - 1,
                                },
                            );
                        }
                    }
                }
            });
        }

        Crawler {
            client,
            requests,
            results,
            progress,
            retry,
        }
    }

    pub fn get_text(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<String, String>> {
        self.get_byte(name, requests)
            .into_iter()
            .map(|result| result.map(|bytes| String::from_utf8(bytes).unwrap()))
            .collect()
    }

    pub fn get_json(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Value, String>> {
        self.get_byte(name, requests)
            .into_iter()
            .map(|result| result.map(|bytes| serde_json::from_slice(&bytes).unwrap()))
            .collect()
    }

    pub fn get_byte(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Vec<u8>, String>> {
        // Initialize the progress bar.
        let total = requests.len();
        let progress = Progress::new(name, total);
        *self.progress.lock().unwrap() = progress;

        // Build and submit requests.
        let requests = requests
            .iter()
            .enumerate()
            .map(|(id, (url, queries))| {
                let request = self
                    .client
                    .lock()
                    .unwrap()
                    .get(*url)
                    .query(queries)
                    .build()
                    .unwrap();
                CrawlerRequest {
                    id,
                    request,
                    retry: self.retry,
                }
            })
            .collect();
        *self.requests.lock().unwrap() = requests;

        info!("Crawler Task \"{name}\" - Start ({total} Requests)");

        // Wait for results.
        loop {
            thread::sleep(Duration::from_secs(1));

            let mut results = self.results.lock().unwrap();
            if results.len() == total {
                info!("Crawler Task \"{name}\" - Complete");

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
        let crawler = Crawler::new(1, 60, Vec::new(), Vec::new(), 1);
        let mut results =
            crawler.get_json("", vec![("https://httpbin.org/user-agent", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let user_agent = json["user-agent"].as_str().unwrap();
        assert_eq!(user_agent, USER_AGENT);
    }

    #[test]
    fn header() {
        let crawler = Crawler::new(1, 60, vec![("K", "V")], Vec::new(), 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/headers", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let value = json["headers"]["K"].as_str().unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn cookie() {
        let crawler = Crawler::new(1, 60, Vec::new(), vec![("K", "V")], 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/cookies", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let value = json["cookies"]["K"].as_str().unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn query() {
        let crawler = Crawler::new(1, 60, Vec::new(), Vec::new(), 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/get", vec![("K", "V")])]);
        let json = results.pop().unwrap().unwrap();
        let value = json["args"]["K"].as_str().unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn get_text() {
        let crawler = Crawler::new(1, 60, Vec::new(), Vec::new(), 1);
        let mut results = crawler.get_text("", vec![("https://httpbin.org/html", Vec::new())]);
        let text = results.pop().unwrap().unwrap();
        assert!(text.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn get_json() {
        let crawler = Crawler::new(1, 60, Vec::new(), Vec::new(), 1);
        let mut results = crawler.get_json("", vec![("https://httpbin.org/json", Vec::new())]);
        let json = results.pop().unwrap().unwrap();
        let value = json["slideshow"]["title"].as_str().unwrap();
        assert_eq!(value, "Sample Slide Show");
    }

    #[test]
    fn get_byte() {
        let crawler = Crawler::new(1, 60, Vec::new(), Vec::new(), 1);
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
