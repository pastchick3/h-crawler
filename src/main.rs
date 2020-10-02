mod crawler;
mod database;
mod error;

use env_logger::Env;
use log::error;
use std::fs;
use std::io::{self, Write};
use structopt::StructOpt;

use crawler::Crawler;
use database::Database;
use error::DisplayableError;

#[derive(StructOpt)]
#[structopt(name = "eh-manager")]
struct Opt {
    ipb_member_id: String,

    ipb_pass_hash: String,

    #[structopt(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let opt = Opt::from_args();
    if opt.debug {
        env_logger::from_env(Env::default().default_filter_or("eh_manager::crawler=debug")).init();
    } else {
        env_logger::from_env(Env::default().default_filter_or("eh_manager::crawler=info")).init();
    }
    let crawler =
        Crawler::new(&opt.ipb_member_id, &opt.ipb_pass_hash).map_err(|err| error!("{}", err))?;
    let database = Database::new().map_err(|err| error!("{}", err))?;

    // Enter the main REPL.
    loop {
        // Print the command prompt.
        print!("> ");
        io::stdout().flush().map_err(|err| error!("{}", err))?;

        // Read the input command.
        let mut command = String::new();
        io::stdin()
            .read_line(&mut command)
            .map_err(|err| error!("{}", err))?;

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
            break Ok(());
        }
        if let Err(err) = execute(&tokens, &crawler, &database).await {
            println!("Error: {}", err);
        }
    }
}

async fn execute(
    tokens: &[String],
    crawler: &Crawler,
    database: &Database,
) -> Result<(), DisplayableError> {
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
                let start = range[0]
                    .parse()
                    .map_err(|err| format!("Can not parse the image range: {}", err))?;
                let end = range[1]
                    .parse()
                    .map_err(|err| format!("Can not parse the image range: {}", err))?;
                if start == 0 || start > end {
                    return Err(DisplayableError::from("Invalid range."));
                }
                (Some(start), Some(end))
            } else {
                (None, None)
            };

            let failed_images = crawler.crawl(artist, title, url, start, end).await?;
            database.add(artist, title, url, start, end)?;
            if !failed_images.is_empty() {
                println!("Fail to download following images:");
                let mut buffer = String::new();
                for page_num in failed_images {
                    buffer.push_str(&format!("{}, ", page_num));
                }
                buffer.pop();
                buffer.pop();
                println!("{}", buffer);
            }

            Ok(())
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

            for result in database.find(artist, title)? {
                println!("{}", result);
            }

            Ok(())
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
                .map_err(|err| error!("{}", err))
                .unwrap();
            let mut command = String::new();
            io::stdin().read_line(&mut command)?;
            command.to_ascii_lowercase();
            if !command.contains('y') {
                return Ok(());
            }

            // Delete galleries.
            for gallery in database.find(artist, title)? {
                fs::remove_dir_all(format!("[{}] {}", gallery.artist, gallery.title))?;
            }
            database.remove(artist, title)?;
            Ok(())
        }
        _ => Err(DisplayableError::from("Unknown command.")),
    }
}

fn tokenize(command: &str) -> Result<Vec<String>, &str> {
    let chars: Vec<_> = command.trim().chars().collect();
    let mut index = 0;
    let mut tokens = Vec::new();

    while index < chars.len() {
        let token = if chars[index] == '"' {
            read_string(&chars, &mut index)?
        } else {
            read_word(&chars, &mut index)
        };
        tokens.push(token);
        skip_whitespaces(&chars, &mut index);
    }

    Ok(tokens)
}

fn read_string(chars: &[char], index: &mut usize) -> Result<String, &'static str> {
    *index += 1; // Skip the opening quotation mark.
    let mut token = String::new();
    let mut escaped = false; // Detect the escaped character.

    while *index < chars.len() {
        if escaped {
            escaped = false;
            token.push(chars[*index]);
        } else if chars[*index] == '"' {
            break;
        } else if chars[*index] == '\\' {
            escaped = true;
        } else {
            token.push(chars[*index]);
        }
        *index += 1;
    }

    if let Some('"') = chars.get(*index) {
        *index += 1; // Skip the closing quotation mark.
        Ok(token)
    } else {
        Err("Unclosed quotation marks.")
    }
}

fn read_word(chars: &[char], index: &mut usize) -> String {
    let mut token = String::new();
    while *index < chars.len() && !chars[*index].is_ascii_whitespace() {
        token.push(chars[*index]);
        *index += 1;
    }
    token
}

fn skip_whitespaces(chars: &[char], index: &mut usize) {
    while *index < chars.len() && chars[*index].is_ascii_whitespace() {
        *index += 1;
    }
}
