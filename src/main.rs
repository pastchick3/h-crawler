mod crawler;
mod database;

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

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    let env = if opt.debug {
        Env::default().default_filter_or("debug")
    } else {
        Env::default().default_filter_or("warn")
    };
    env_logger::from_env(env).init();
    let database = Database::new(&opt.username, &opt.password, &opt.resource);

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
        match tokens[0] {
            "status" => {
                for line in database.status() {
                    println!("{}", line);
                }
            }
            "add" => {
                if tokens.len() <= 4 {
                    println!("Error: Insufficient arguments.");
                    continue;
                }
                let artist = tokens[1];
                let title = tokens[2];
                let url = tokens[3];
                let range = if let Some(range) = tokens.get(4) {
                    let range: Vec<_> = range.split("-").collect();
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
                if tokens.len() <= 3 {
                    println!("Error: Insufficient arguments.");
                    continue;
                }
                let artist = match tokens[1] {
                    "*" => None,
                    s => Some(s),
                };
                let title = match tokens[2] {
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
                if !buffer.contains("y") {
                    continue;
                }

                if let Err(err) = database.remove(artist, title) {
                    println!("Error: {}.", err);
                    continue;
                }
            }
            "find" => {
                if tokens.len() <= 3 {
                    println!("Error: Insufficient arguments.");
                    continue;
                }
                let artist = match tokens[1] {
                    "*" => None,
                    s => Some(s),
                };
                let title = match tokens[2] {
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

fn tokenize(command: &str) -> Result<Vec<&str>, &str> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut token_flag = false;
    let mut quotation_flag = false;
    let mut back_slash_flag = false;
    for (j, ch) in command.char_indices() {
        if let '\\' = ch {
            back_slash_flag = true;
            continue;
        } else {
            back_slash_flag = false;
        }

        match ch {
            '"' => {
                if token_flag && quotation_flag {
                    tokens.push(&command[i..j]);
                    token_flag = false;
                    quotation_flag = false;
                } else if token_flag && !quotation_flag {
                    // This happens for input like `term_a"term_b"`.
                    tokens.push(&command[i..j]);
                    i = j + 1;
                    token_flag = true;
                    quotation_flag = true;
                } else if !token_flag && quotation_flag {
                    return Err("Enter");
                } else {
                    i = j + 1;
                    token_flag = true;
                    quotation_flag = true;
                }
            }
            c if !quotation_flag => {
                if c.is_ascii_whitespace() {
                    tokens.push(&command[i..j]);
                    token_flag = false;
                } else {
                    i = j;
                    token_flag = true;
                }
            }
            _ => (),
        }
    }
    if token_flag {
        tokens.push(&command[i..]);
    }
    if quotation_flag {
        Err("Unclosed double quotation mark.")
    } else {
        Ok(tokens)
    }
}
