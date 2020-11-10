mod crawler;

use crawler::Crawler;
use serde_derive::Deserialize;
use std::fs;
use std::io::{self, Write};
use toml;

const EH_CREDENTIAL: &str = "./eh-credential";

#[derive(Deserialize)]
pub struct Credential {
    ipb_member_id: String,
    ipb_pass_hash: String,
}

#[tokio::main]
async fn main() {
    let credential_str = fs::read_to_string(EH_CREDENTIAL).unwrap();
    let credential: Credential = toml::from_str(&credential_str).unwrap();

    let mut crawler = Crawler::new(credential);

    // Enter the main REPL.
    loop {
        // Print the command prompt.
        print!("> ");
        io::stdout().flush().unwrap();

        // Read the input command.
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        let mut command = command.split_whitespace();

        match command.next() {
            Some("crawl") => {
                let url = match command.next() {
                    Some(url) => url,
                    None => {
                        println!("url not found.");
                        continue;
                    }
                };
                let (start, end) = match command.next() {
                    Some(range) => {
                        let range: Vec<_> = range.split('-').collect();
                        if range.len() != 2 {
                            println!("Invalid range.");
                            continue;
                        }
                        let start = match range[0].parse() {
                            Ok(start) => start,
                            Err(err) => {
                                println!("Can not parse the image range: {}", err);
                                continue;
                            }
                        };
                        let end = match range[1].parse() {
                            Ok(end) => end,
                            Err(err) => {
                                println!("Can not parse the image range: {}", err);
                                continue;
                            }
                        };
                        if start == 0 || start > end {
                            println!("Invalid range.");
                            continue;
                        }
                        (Some(start), Some(end))
                    }
                    None => (None, None),
                };
                let id = crawler.crawl(url, start, end);
                println!("Start to crawl gallery `{}`", id);
            }
            Some("status") => {
                let status = crawler.status(command.next());
                println!("{}", status);
            }
            Some("cancel") => {
                if let Some(id) = command.next() {
                    // Require remove comfirmation.
                    let status = crawler.status(Some(id));
                    println!("Are you sure to cancel: \n{}", status);
                    print!("Press [y/n]: ");
                    io::stdout().flush().unwrap();
                    let mut command = String::new();
                    io::stdin().read_line(&mut command).unwrap();
                    command.to_ascii_lowercase();
                    if command.starts_with('y') {
                        crawler.cancel(id);
                        println!("Gallery `{}` is cancelled.", id);
                    }
                } else {
                    println!("Gallery ID not found.");
                }
            }
            Some("exit") => break,
            Some(_) => println!("Invalid Commandd"),
            None => continue,
        }
    }
}
