use clap::{self, command, Arg, ArgAction};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File},
    io::{self, stdin, Read, Write},
    path::{Path, PathBuf},
};
use path_absolutize::Absolutize;

const CONFIG_FILE: &'static str = ".rmrs.toml";

#[derive(Deserialize, Serialize)]
struct Config {
    location: String,
}

#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(unused_assignments)]
fn main() -> io::Result<()> {
    let mut trash_location = String::new();
    if let Ok(location) = env::var("TRASH") {
        trash_location = location;
    } else {
        trash_location = proc_toml().unwrap();
    }
    run();
    Ok(())
}

fn proc_toml() -> Result<String, String> {
    let mut p = PathBuf::new();
    p.push(env::var("HOME").unwrap());
    p.push(CONFIG_FILE);
    if Path::exists(&p) {
        let mut conf = File::open(&p).unwrap();
        let mut content = String::new();
        conf.read_to_string(&mut content).unwrap();
        if let Ok(config) = toml::from_str::<Config>(&content) {
            return Ok(config.location);
        } else {
            match fs::remove_file(&p) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{e}");
                }
            }
            Err(format!("broken \"{}\", please rerun", p.to_string_lossy()))
        }
    } else {
        let mut conf = File::create(&p).unwrap();
        println!(
            "\t\tLooks like you haven't used rmrs yet\n\
            \t\tThe default trash location would be \"{}/.trash\"\n\
        \t\tor you may input customized trash location(absolute):",
            env::var("HOME").unwrap()
        );
        let mut user_input: String = String::new();
        stdin().read_line(&mut user_input).unwrap();
        user_input.pop();
        let mut config: Config = Config {
            location: String::from(""),
        };
        if is_valid_path(&user_input) {
            config.location = user_input;
        } else {
            config.location = format!("{}/.trash", env::var("HOME").unwrap());
        }
        let content = toml::to_string(&config).unwrap();
        conf.write_all(content.as_bytes()).unwrap();
        conf.flush().unwrap();
        return Ok(config.location);
    }
}
fn is_valid_path(p: &str) -> bool {
    let re = Regex::new(r"^/[/?\.?\w]+\w$").unwrap();
    match re.find(p) {
        Some(m) => {
            return m.as_str().eq(p);
        }
        None => {
            return false;
        }
    }
}
fn run() {
    let matches = command!()
        .arg(
            Arg::new("file(s)")
                .action(ArgAction::Append)
                .required(false)
                .help("ordinary file(s) you want to delete"),
        )
        .arg(
            Arg::new("location")
                .action(ArgAction::Set)
                .required(false)
                .short('l')
                .long("location")
                .help("set trash location"),
        )
        .arg(
            Arg::new("forever")
                .action(ArgAction::SetTrue)
                .required(false)
                .short('f')
                .long("forever")
                .help("delete forever from disc"),
        )
        .arg(
            Arg::new("clear")
                .action(ArgAction::SetTrue)
                .required(false)
                .short('c')
                .long("clear")
                .help("clear trash"),
        )
        .get_matches();
    let args = matches.get_many::<String>("file(s)").unwrap_or_default().map(|v| v.as_str()).collect::<Vec<_>>();
    for arg in args {
        let p = Path::new(arg);
        fs::remove_file(p.absolutize().unwrap().to_str().unwrap()).unwrap();
    }
}
