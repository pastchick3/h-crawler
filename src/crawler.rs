use log::{info, debug};
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
    visible: bool,
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} => {}/{}", self.name, self.done, self.total)
    }
}

impl Progress {
    fn new(name: &str, total: usize, visible: bool) -> Self {
        let progress = Progress {
            name: String::from(name),
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

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/99.0.4844.51 Safari/537.36 Edg/99.0.1150.39",
);
#[derive(Debug)]
struct InnerRequest {
    id: usize,
    request: Request,
    retry: usize,
}
#[derive(Debug)]
struct InnerResult {
    id: usize,
    result: Result<Response, Error>,
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
        headers: Vec<(&str, &str)>,
        cookies: Vec<(&str, &str)>,
        retry: usize,
    ) -> Self {
        info!("Build crawler - concurrency: {concurrency}, timeout: {timeout}, headers: {headers:?}, cookies: {cookies:?}, retry: {retry}");

        let requests = Arc::new(Mutex::new(Vec::new()));
        let results = Arc::new(Mutex::new(Vec::new()));
        let progress = Arc::new(Mutex::new(Progress::new("", 0, false)));

        for _ in 0..concurrency {
            
        
            let requests = requests.clone();
            let results = results.clone();
            let progress = progress.clone();
            thread::spawn(move || loop {
                let task = {requests.lock().unwrap().pop()};
                
                match task {
                    Some(InnerRequest { id, request, mut retry }) => match request.clone().call() {
                            Ok(resp) => {
                                
                                info!("Request succeed - id: {id}, request: {request:?}");
                                let mut prog = progress.lock().unwrap();
                                prog.make_progress();
                                prog.print_progress();
                                results.lock().unwrap().push(InnerResult {
                                    id,
                                    result: Ok(resp),
                                });
                            }
                            Err(err) => {
                                
                if retry == 0 {
                    
                                info!("Request fail - id: {id}, request: {request:?}, error: {err:?}");
                                    results.lock().unwrap().push(InnerResult {
                                        id,
                                        result: Err(err),
                                    });
                                
                                } else {
                                    retry -= 1;
                                    debug!("Retry request - id: {id}, request: {request:?}, retry: {retry}");
                                    requests
                                        .lock()
                                        .unwrap()
                                        .insert(0, InnerRequest { id, request, retry });
                                }}
                        }
                    None => thread::sleep(Duration::from_millis(1000)),
                }
            });
        }
        let headers: Vec<_> = headers.into_iter().map(|(n, v)| (n.to_string(), v.to_string())).collect();
        let add_headers = move |mut request: Request, next: MiddlewareNext| {
            for (name, value) in headers.clone() {
                request = request.set(&name, &value);
            }
            next.handle(request)
        };
        let cookies: Vec<_> = cookies.into_iter().map(|(n, v)| (n.to_string(), v.to_string())).collect();
        let add_cookies = move |request: Request, next: MiddlewareNext| {
            let mut cookie_str = String::new();
            for (name, value) in cookies.clone() {
                cookie_str.push_str(&format!("{name}={value};"));
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
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<String, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|rslt| match rslt {
                Ok(resp) => match resp.into_string() {
                    Ok(resp) => Ok(resp),
                    Err(err) => Err(err.to_string()),
                },
                Err(err) => Err(err),
            })
            .collect()
    }

    pub fn get_json(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Value, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|rslt| match rslt {
                Ok(resp) => match resp.into_json() {
                    Ok(resp) => Ok(resp),
                    Err(err) => Err(err.to_string()),
                },
                Err(err) => Err(err),
            })
            .collect()
    }

    pub fn get_byte(
        &self,
        name: &str,
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Vec<u8>, String>> {
        self.get(name, requests)
            .into_iter()
            .map(|rslt| {
                let mut bytes = Vec::new();
                match rslt {
                    Ok(resp) => match resp.into_reader().read_to_end(&mut bytes) {
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
        requests: Vec<(&str, Vec<(&str, &str)>)>,
    ) -> Vec<Result<Response, String>> {
        let total = requests.len();
        let progress = Progress::new(name, total, !name.is_empty());
        *self.progress.lock().unwrap() = progress;
        
        
        let requests: Vec<_> = requests.into_iter().enumerate().map(|(id, (url, queries))| {
            let mut request = self.agent.get(url);
            for (name, value) in queries {
                request = request.query(name, value);
            }
            InnerRequest {
                id,
                request,
                retry: self.retry,
            }
        }).collect();
        {*self.requests.lock().unwrap() = requests;
        }
        // {self.requests.lock();}
        loop {
            {
            let mut results = self.results.lock().unwrap();
            if results.len() == total {
                    let mut results = mem::take(&mut *results);
                    results.sort_unstable_by_key(|rslt| rslt.id);
                    self.progress.lock().unwrap().finish();
                    return results
                        .into_iter()
                        .map(|rslt| rslt.result.map_err(|err| err.to_string()))
                        .collect();
            }}
            {
                thread::sleep(Duration::from_millis(300));
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
        let mut results = crawler.get_json(
            "",
            vec![("https://httpbin.org/user-agent", Vec::new())],
        );
        let response = results.pop().unwrap().unwrap();
        let value = response["user-agent"].as_str().unwrap();
        assert_eq!(value, "USER_AGENT");
    }

    // #[test]
    // fn header() {
    //     let crawler = Crawler::new(
    //         2,
    //         15,
    //         vec![(String::from("K"), String::from("V"))],
    //         Vec::new(),
    //         1,
    //     );
    //     let results = crawler.get_json(
    //         "",
    //         vec![(String::from("https://httpbin.org/headers"), Vec::new())],
    //     );
    //     let value = results[0].as_ref().unwrap()["headers"]["K"]
    //         .as_str()
    //         .unwrap();
    //     assert_eq!(value, "V");
    // }

    // #[test]
    // fn cookie() {
    //     let crawler = Crawler::new(
    //         2,
    //         15,
    //         Vec::new(),
    //         vec![(String::from("K"), String::from("V"))],
    //         1,
    //     );
    //     let results = crawler.get_json(
    //         "",
    //         vec![(String::from("https://httpbin.org/cookies"), Vec::new())],
    //     );
    //     let cookie = results[0].as_ref().unwrap()["cookies"].as_str().unwrap();
    //     assert_eq!(cookie, "K=V;");
    // }

    // #[test]
    // fn query() {
    //     let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
    //     let results = crawler.get_json(
    //         "",
    //         vec![(
    //             String::from("https://httpbin.org/get"),
    //             vec![(String::from("K"), String::from("V"))],
    //         )],
    //     );
    //     let value = results[0].as_ref().unwrap()["args"]["K"].as_str().unwrap();
    //     assert_eq!(value, "V");
    // }

    // #[test]
    // fn get_text() {
    //     let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
    //     let results = crawler.get_text(
    //         "",
    //         vec![(String::from("https://httpbin.org/html"), Vec::new())],
    //     );
    //     let value = results[0].as_ref().unwrap();
    //     assert!(value.starts_with("<!DOCTYPE html>"));
    // }

    // #[test]
    // fn get_json() {
    //     let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
    //     let results = crawler.get_json(
    //         "",
    //         vec![(String::from("https://httpbin.org/json"), Vec::new())],
    //     );
    //     let value = results[0].as_ref().unwrap()["slideshow"]["title"]
    //         .as_str()
    //         .unwrap();
    //     assert_eq!(value, "Sample Slide Show");
    // }

    // #[test]
    // fn get_byte() {
    //     let crawler = Crawler::new(2, 15, Vec::new(), Vec::new(), 1);
    //     let results = crawler.get_byte(
    //         "",
    //         vec![(
    //             String::from("https://httpbin.org/base64/SFRUUEJJTiBpcyBhd2Vzb21l"),
    //             Vec::new(),
    //         )],
    //     );
    //     let value = results[0].as_ref().unwrap();
    //     assert_eq!(value, "HTTPBIN is awesome".as_bytes());
    // }
}
