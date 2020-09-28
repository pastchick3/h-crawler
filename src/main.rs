mod crawler;
mod database;
mod gallery;

use env_logger::Env;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

use database::Database;

#[derive(StructOpt)]
#[structopt(name = "eh-manager")]
struct Opt {
    username: String,

    password: String,

    #[structopt(long, parse(from_os_str), default_value = ".")]
    resource: PathBuf,

    #[structopt(long)]
    debug: bool,
}

fn main() {
    let opt = Opt::from_args();
    let env = if opt.debug {
        Env::default().default_filter_or("debug")
    } else {
        Env::default().default_filter_or("warn")
    };
    env_logger::from_env(env).init();
    let database = match Database::new(opt.username, opt.password, opt.resource) {
        Ok(database) => database,
        Err(err) =>{
            println!("Error: {}", err);
            return;
        }
    };

    repl(&database);
}

fn repl(database: &Database) {
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
        match tokens[0].as_str() {
            "status" => {
                for line in database.status() {
                    println!("{}", line);
                }
            }
            "add" => {
                if tokens.len() < 4 {
                    println!("Error: Insufficient arguments.");
                    continue;
                }
                let artist = &tokens[1];
                let title = &tokens[2];
                let url = &tokens[3];
                let range = if let Some(range) = tokens.get(4) {
                    let range: Vec<_> = range.split('-').collect();
                    if range.len() != 2 {
                        println!("Error: Invalid range.");
                        continue;
                    }
                    let start = match range[0].parse() {
                        Ok(start) => start,
                        Err(err) => {
                            println!("Error: {}", err);
                            continue;
                        }
                    };
                    let end = match range[1].parse() {
                        Ok(end) => end,
                        Err(err) => {
                            println!("Error: {}.", err);
                            continue;
                        }
                    };
                    Some((start, end))
                } else {
                    None
                };
                if let Err(err) = database.add(artist, title, url, range) {
                    println!("Error: {}.", err);
                    continue;
                }
            }
            "remove" => {
                if tokens.len() < 3 {
                    println!("Error: Insufficient arguments.");
                    continue;
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
                for record in database.find(artist, title) {
                    println!("{}", record)
                }
                print!("Press [y/n]: ");
                io::stdout()
                    .flush()
                    .expect("Error: Unable to flush the REPL output.");
                let mut buffer = String::new();
                if let Err(err) = io::stdin().read_line(&mut buffer) {
                    println!("Error: {}", err);
                    continue;
                }
                buffer.to_ascii_lowercase();
                if !buffer.contains('y') {
                    continue;
                }

                if let Err(err) = database.remove(artist, title) {
                    println!("Error: {}.", err);
                    continue;
                }
            }
            "find" => {
                if tokens.len() < 3 {
                    println!("Error: Insufficient arguments.");
                    continue;
                }
                let artist = match tokens[1].as_str() {
                    "*" => None,
                    s => Some(s),
                };
                let title = match tokens[2].as_str() {
                    "*" => None,
                    s => Some(s),
                };
                for record in database.find(artist, title) {
                    println!("{}", record)
                }
            }
            "exit" => break,
            _ => println!("Unknown command."),
        }
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
