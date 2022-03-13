use log::{error, info, warn};
use serde_json::Value;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, Read, Write};
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use ureq::{Agent, AgentBuilder, Error, MiddlewareNext, Request, Response};

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/99.0.4844.51 Safari/537.36 Edg/99.0.1150.39",
);

struct InnerRequest {
    id: usize,
    request: Request,
    retry: usize,
}

struct InnerResult {
    id: usize,
    result: Result<Response, Error>,
}

struct Progress {
    title: String,
    done: usize,
    total: usize,
    visible: bool,
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} => {}/{}", self.title, self.done, self.total)
    }
}

impl Progress {
    fn new(title: &str, total: usize, visible: bool) -> Self {
        let progress = Progress {
            title: title.to_string(),
            done: 0,
            total,
            visible,
        };
        progress.print_progress();
        progress
    }

    fn make_progress(&mut self) {
        self.done += 1;
        self.print_progress();
    }

    fn print_progress(&self) {
        if self.visible {
            print!("\r{}", self);
            io::stdout().flush().unwrap();
        }
    }

    fn finish(&self) {
        if self.visible {
            println!("");
            io::stdout().flush().unwrap();
        }
    }
}

pub struct Crawler {
    agent: Agent,
    requests: Arc<Mutex<Vec<InnerRequest>>>,
    results: Arc<Mutex<Vec<InnerResult>>>,
    retry: usize,
    progress: Arc<Mutex<Progress>>,
}

impl Crawler {
    pub fn new(
        concurrency: usize,
        timeout: u64,
        headers: Vec<(String, String)>,
        cookies: Vec<(String, String)>,
        retry: usize,
    ) -> Self {
        info!("Crawler built - concurrency: {concurrency}, timeout: {timeout}, headers: {headers:?}, cookies: {cookies:?}, retry: {retry}");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let results = Arc::new(Mutex::new(Vec::new()));
        let progress = Arc::new(Mutex::new(Progress::new("", 0, false)));

        for _ in 0..concurrency {
            let requests = requests.clone();
            let results = results.clone();
            let progress = progress.clone();
            thread::spawn(move || loop {
                match requests.lock().unwrap().pop() {
                    Some(InnerRequest { id, request, retry }) => {
                        match request.clone().call() {
                            Ok(response) => {
                                info!("Request succeed - id: {id}, request: {request:?}");
                                let mut prog = progress.lock().unwrap();
                                prog.make_progress();
                                prog.print_progress();
                                results.lock().unwrap().push(InnerResult {
                                    id,
                                    result: Ok(response),
                                })
                            }
                            Err(error) => {
                                if retry == 0 {
                                    error!("Request fail - id: {id}, request: {request:?}, error: {error:?}");
                                    results.lock().unwrap().push(InnerResult {
                                        id,
                                        result: Err(error),
                                    });
                                } else {
                                    let retry = retry - 1;
                                    warn!("Request retry - id: {id}, request: {request:?}, retry: {retry}");
                                    requests
                                        .lock()
                                        .unwrap()
                                        .insert(0, InnerRequest { id, request, retry });
                                }
                            }
                        };
                    }
                    None => thread::sleep(Duration::from_millis(200)),
                }
            });
        }
        let add_headers = move |mut request: Request, next: MiddlewareNext| {
            for (name, value) in headers.clone() {
                request = request.set(&name, &value);
            }
            next.handle(request)
        };
        let add_cookies = move |request: Request, next: MiddlewareNext| {
            let mut cookie_str = String::new();
            for (name, value) in cookies.clone() {
                cookie_str.push_str(&format!("{name}:{value};"));
            }
            next.handle(request.set("Cookie", &cookie_str))
        };
        let agent = AgentBuilder::new()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(timeout))
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
        requests: Vec<(String, Vec<(String, String)>)>,
    ) -> Vec<Result<String, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|result| match result {
                Ok(response) => match response.into_string() {
                    Ok(string) => Ok(string),
                    Err(err) => Err(err.to_string()),
                },
                Err(err) => Err(err),
            })
            .collect()
    }

    pub fn get_json(
        &self,
        name: &str,
        requests: Vec<(String, Vec<(String, String)>)>,
    ) -> Vec<Result<Value, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|result| match result {
                Ok(response) => match response.into_json() {
                    Ok(value) => Ok(value),
                    Err(err) => Err(err.to_string()),
                },
                Err(err) => Err(err),
            })
            .collect()
    }

    pub fn get_byte(
        &self,
        name: &str,
        requests: Vec<(String, Vec<(String, String)>)>,
    ) -> Vec<Result<Vec<u8>, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|result| {
                let mut bytes = Vec::new();
                match result {
                    Ok(response) => match response.into_reader().read_to_end(&mut bytes) {
                        Ok(_) => Ok(bytes),
                        Err(err) => Err(err.to_string()),
                    },
                    Err(err) => Err(err),
                }
            })
            .collect()
    }

    fn get(
        &self,
        name: &str,
        requests: Vec<(String, Vec<(String, String)>)>,
    ) -> Vec<Result<Response, String>> {
        let num = requests.len();
        let progress = Progress::new(name, num, !name.is_empty());
        progress.print_progress();
        *self.progress.lock().unwrap() = progress;
        for (id, (url, queries)) in requests.into_iter().enumerate() {
            let mut request = self.agent.get(&url);
            for (param, value) in queries {
                request = request.query(&param, &value);
            }
            self.requests.lock().unwrap().push(InnerRequest {
                id,
                request,
                retry: self.retry,
            });
        }
        loop {
            let mut results = self.results.lock().unwrap();
            match results.len() == num {
                true => {
                    let mut r = mem::take(&mut *results);
                    r.sort_unstable_by_key(|res| res.id);
                    self.progress.lock().unwrap().finish();
                    return r
                        .into_iter()
                        .map(|result| result.result.map_err(|err| err.to_string()))
                        .collect();
                }
                false => thread::sleep(Duration::from_millis(200)),
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
        let results = crawler.get_json(
            "",
            vec![(String::from("https://httpbin.org/user-agent"), Vec::new())],
        );
        let value = results[0].as_ref().unwrap()["user-agent"].as_str().unwrap();
        assert_eq!(value, USER_AGENT);
    }

    #[test]
    fn header() {
        let crawler = Crawler::new(
            2,
            15,
            vec![(String::from("K"), String::from("V"))],
            Vec::new(),
            1,
        );
        let results = crawler.get_json(
            "",
            vec![(String::from("https://httpbin.org/headers"), Vec::new())],
        );
        let value = results[0].as_ref().unwrap()["headers"]["K"]
            .as_str()
            .unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn cookie() {
        let crawler = Crawler::new(
            2,
            15,
            Vec::new(),
            vec![(String::from("K"), String::from("V"))],
            1,
        );
        let results = crawler.get_json(
            "",
            vec![(String::from("https://httpbin.org/cookies"), Vec::new())],
        );
        let cookie = results[0].as_ref().unwrap()["cookies"].as_str().unwrap();
        assert_eq!(cookie, "K=V;");
    }

    #[test]
    fn query() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let results = crawler.get_json(
            "",
            vec![(
                String::from("https://httpbin.org/get"),
                vec![(String::from("K"), String::from("V"))],
            )],
        );
        let value = results[0].as_ref().unwrap()["args"]["K"].as_str().unwrap();
        assert_eq!(value, "V");
    }

    #[test]
    fn get_text() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let results = crawler.get_text(
            "",
            vec![(String::from("https://httpbin.org/html"), Vec::new())],
        );
        let value = results[0].as_ref().unwrap();
        assert!(value.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn get_json() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let results = crawler.get_json(
            "",
            vec![(String::from("https://httpbin.org/json"), Vec::new())],
        );
        let value = results[0].as_ref().unwrap()["slideshow"]["title"]
            .as_str()
            .unwrap();
        assert_eq!(value, "Sample Slide Show");
    }

    #[test]
    fn get_byte() {
        let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
        let results = crawler.get_byte(
            "",
            vec![(
                String::from("https://httpbin.org/base64/SFRUUEJJTiBpcyBhd2Vzb21l"),
                Vec::new(),
            )],
        );
        let value = results[0].as_ref().unwrap();
        assert_eq!(value, "HTTPBIN is awesome".as_bytes());
    }
}
