
use std::io::{self, Write, Read};
use ureq::{AgentBuilder, Agent, Response};
use std::time::Duration;
use log::{info, debug};

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/81.0.4044.138 ",
    "Safari/537.36 Edg/81.0.416.72",
);



pub struct Crawler {
    agent: Agent,
    retry: usize,
}

impl Crawler {
    pub fn new(timeout: u64, retry:usize, concurrency: usize, cookies: Vec<(String, String)>) -> Self {
        let agent = 
        AgentBuilder::new()
        //.cookie_store()
        .timeout(Duration::from_secs(timeout))
        .user_agent(USER_AGENT)
      .build();
     
        Crawler {
            agent,
            retry,
        }
    }

    pub fn get(
        &self,
        url: &str,
        queries: Vec<(String, String)>,
    ) -> Result<Vec<u8>, String> {
        let resp = self._get(url, queries)?;
        let mut buf = Vec::new();
        resp.into_reader().read_to_end(&mut buf);
        return Ok(buf);
    }

    pub fn batch(
        &self,
        tasks: Vec<(&str, Vec<(String, String)>)>
    ) -> Vec<Result<Vec<u8>, String>> {
        tasks.into_iter().map(|(u, q)| self.get(u, q)).collect()
    }

    pub fn batch_text(
        &self,
        tasks: Vec<(&str, Vec<(String, String)>)>
    ) -> Vec<Result<String, String>> {
        tasks.into_iter().map(|(u, q)| self.get_text(u, q)).collect()
    }

    pub fn get_text(
        &self,
        url: &str,
        queries: Vec<(String, String)>,
    ) -> Result<String, String> {
        let resp = self._get(url, queries)?;
        resp.into_string().map_err(|e| format!("{e}"))
    }

    pub fn _get(
        &self,
        url: &str,
        queries: Vec<(String, String)>,
    ) -> Result<Response, String> {
        let mut error = String::new();
        for r in 0..=self.retry {
            let mut req = self.agent.get(url);
            for (param, value) in &queries {
                req = req.query(&param, &value);
            }
            match req.call() {
                Ok(resp) => {
                    debug!("Request `{url}` (retry={r}) succeeds");
                    return Ok(resp)
                }
                Err(err) => {
                    debug!("Request `{url}` (retry={r}) fails: {err}");
                    error = err.to_string();
                }
            };
        }
        return Err(format!("{error}"));
    }
}
