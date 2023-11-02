use clap::{self, command, Arg, ArgAction};
use path_absolutize::Absolutize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, rename, File, remove_file},
    io::{self, stdin, Read, Write},
    path::{Path, PathBuf},
};

const CONFIG_FILE: &'static str = ".rmrs.toml";

#[derive(Deserialize, Serialize)]
struct Config {
    location: String,
}
/// A struct to store args
#[derive(Debug)]
struct UserCommand<T>
where
    T: AsRef<str>,
{
    files: Vec<T>,
    f: bool,
    c: bool,
    z: bool,
}

impl<T> UserCommand<T>
where
    T: AsRef<str>,
{
    fn new(files: Vec<T>, f: bool, c: bool, z: bool) -> UserCommand<T> {
        Self { files, f, c, z }
    }
    fn is_empty(&self) -> bool {
        self.files.is_empty() & !self.f & !self.c & !self.z
    }
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
    let path_location = Path::new(trash_location.as_str());
    if !path_location.exists() {
        fs::create_dir_all(path_location)?;
    }
    env::set_var("tl", trash_location);
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
        .arg(
            Arg::new("regret")
                .action(ArgAction::SetTrue)
                .required(false)
                .short('z')
                .exclusive(true)
                .help("recover last deleted file or directory if trash hasn't been cleared"),
        )
        .get_matches();
    let args = matches
        .get_many::<String>("file(s)")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();
    let f = matches.get_flag("forever");
    let c = matches.get_flag("clear");
    let z = matches.get_flag("regret");
    let user_args = UserCommand::new(args,f,c,z);
    println!("{:?}", user_args);
    println!("{}", user_args.is_empty());
    move_to_trash(user_args.files);
}
fn move_to_trash(files: Vec<&str>) {
    let to: PathBuf = PathBuf::from(env::var("tl").unwrap());
    for file in files {
        let file_path_abs = Path::new(file).absolutize().unwrap();
        rename(file_path_abs, to.join(file)).unwrap();
    }
}
fn move_forever(files: Vec<&str>) {
        let to: PathBuf = PathBuf::from(env::var("tl").unwrap());
    for file in files {
        let file_path_abs = Path::new(file).absolutize().unwrap();
        remove_file(file_path_abs).unwrap();
    }
}
fn log() {
    !todo!()
}
fn regret() {
    todo!()
}
fn clear() {
    todo!()
}