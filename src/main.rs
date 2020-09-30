mod crawler;
mod database;
mod error;

use std::io::{self, Write};
use structopt::StructOpt;
use std::fs;

use crawler::Crawler;
use database::{Database, Gallery};
use error::DisplayableError;

#[derive(StructOpt)]
#[structopt(name = "eh-manager")]
struct Opt {
    username: String,

    password: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let opt = Opt::from_args();
    let crawler = match Crawler::new(&opt.username, &opt.password) {
        Ok(crawler) => crawler,
        Err(err) => {
            println!("Error: {}", err);
            return;
        }
    };
    let database = match Database::new() {
        Ok(database) => database,
        Err(err) => {
            println!("Error: {}", err);
            return;
        }
    };

    loop {
        // Print the command prompt.
        print!("> ");
        io::stdout()
            .flush()
            .expect("Error: Unable to flush the REPL output.");

        // Read the input command.
        let mut command = String::new();
        if let Err(err) = io::stdin().read_line(&mut command) {
            println!("Error: {}", err);
            continue;
        }

        // Tokenize the command.
        command.make_ascii_lowercase();
        let tokens = match tokenize(&command) {
            Ok(tokens) => tokens,
            Err(err) => {
                println!("Error: {}", err);
                continue;
            }
        };

        // Execute the command.
        if tokens.is_empty() {
            continue;
        }
        if tokens[0] == "exit" {
            break;
        }
        match execute(&tokens, &crawler, &database).await {
            Ok(results) => {
                for result in results {
                    println!("{}", result);
                }
            }
            Err(err) => {
                println!("Error: {}", err);
            }
        }
    }
}

async fn execute(
    tokens: &[String],
    crawler: &Crawler,
    database: &Database,
) -> Result<Vec<Gallery>, DisplayableError> {
    match tokens[0].as_str() {
        "add" => {
            if tokens.len() < 4 {
                return Err(DisplayableError::from("Insufficient arguments."));
            }
            let artist = &tokens[1];
            let title = &tokens[2];
            let url = &tokens[3];
            let (start, end) = if let Some(range) = tokens.get(4) {
                let range: Vec<_> = range.split('-').collect();
                if range.len() != 2 {
                    return Err(DisplayableError::from("Invalid range."));
                }
                let start = range[0].parse()?;
                let end = range[1].parse()?;
                (Some(start), Some(end))
            } else {
                (None, None)
            };

            crawler.crawl(artist, title, url, start, end).await?;
            database.add(artist, title, url, start, end)?;
            Ok(Vec::new())
        }
        "remove" => {
            if tokens.len() < 3 {
                return Err(DisplayableError::from("Insufficient arguments."));
            }
            let artist = match tokens[1].as_str() {
                "*" => None,
                s => Some(s),
            };
            let title = match tokens[2].as_str() {
                "*" => None,
                s => Some(s),
            };

            // Require remove comfirmation.
            println!("Are you sure to remove:");
            database
                .find(artist, title)?
                .iter()
                .for_each(|g| println!("{}", g));
            print!("Press [y/n]: ");
            io::stdout()
                .flush()
                .expect("Error: Unable to flush the REPL output.");
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            buffer.to_ascii_lowercase();
            if !buffer.contains('y') {
                return Ok(Vec::new());
            }

            for gallery in database.find(artist, title)? {
                fs::remove_dir_all(format!("[{}] {}", gallery.artist, gallery.title))?;
            }
            database.remove(artist, title)?;
            Ok(Vec::new())
        }
        "find" => {
            if tokens.len() < 3 {
                return Err(DisplayableError::from("Insufficient arguments."));
            }
            let artist = match tokens[1].as_str() {
                "*" => None,
                s => Some(s),
            };
            let title = match tokens[2].as_str() {
                "*" => None,
                s => Some(s),
            };
            Ok(database.find(artist, title)?)
        }
        _ => Err(DisplayableError::from("Unknown command.")),
    }
}

fn tokenize(command: &str) -> Result<Vec<String>, &str> {
    let chars: Vec<_> = command.trim().chars().collect();
    let mut index = 0;
    let mut tokens = Vec::new();

    while index < chars.len() {
        let (buffer, i) = if chars[index] == '"' {
            read_string(&chars, index)?
        } else {
            read_word(&chars, index)
        };
        tokens.push(buffer);
        index = skip_whitespaces(&chars, i);
    }

    Ok(tokens)
}

fn skip_whitespaces(chars: &[char], mut index: usize) -> usize {
    while index < chars.len() && chars[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn read_string(chars: &[char], mut index: usize) -> Result<(String, usize), &'static str> {
    index += 1; // Skip the opening quotation mark.
    let mut buffer = String::new();
    let mut back_slash_flag = false;
    let mut closed = false;

    while index < chars.len() {
        if back_slash_flag {
            back_slash_flag = false;
            buffer.push(chars[index]);
            index += 1;
        } else if chars[index] == '"' {
            closed = true;
            index += 1;
            break;
        } else if chars[index] == '\\' {
            back_slash_flag = true;
            index += 1;
        } else {
            buffer.push(chars[index]);
            index += 1;
        }
    }

    if closed {
        Ok((buffer, index))
    } else {
        Err("Unclosed quotation marks.")
    }
}

fn read_word(chars: &[char], mut index: usize) -> (String, usize) {
    let mut buffer = String::new();
    while index < chars.len() && !chars[index].is_ascii_whitespace() {
        buffer.push(chars[index]);
        index += 1;
    }
    (buffer, index)
}
