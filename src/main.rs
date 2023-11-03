use clap::{self, command, Arg, ArgAction};
use path_absolutize::Absolutize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, remove_file, rename, File, OpenOptions},
    io::{self, stdin, Read, Write},
    path::{Path, PathBuf},
};
use time as Dime;
use Dime::{format_description, macros::offset};
const CONFIG_FILE: &'static str = ".rmrs.toml";

#[derive(Deserialize, Serialize)]
struct Config {
    location: String,
    confirm: String,
}
/// A struct to store args
#[derive(Debug)]
struct UserCommand<T>
where
    T: AsRef<Path>,
{
    files: Vec<T>,
    dirs: Vec<T>,
    f: bool,
    c: bool,
    z: bool,
}

impl<T> UserCommand<T>
where
    T: AsRef<Path>,
{
    fn new(files: Vec<T>, dirs: Vec<T>, f: bool, c: bool, z: bool) -> UserCommand<T> {
        Self {
            files,
            dirs,
            f,
            c,
            z,
        }
    }
}
#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(unused_assignments)]
fn main() -> io::Result<()> {
    let mut trash_home = String::new();
    if let Ok(location) = env::var("TRASH_HOME") {
        trash_home = location;
    } else {
        trash_home = proc_toml().unwrap();
    }
    let path_location = Path::new(trash_home.as_str());
    if !path_location.exists() {
        fs::create_dir_all(path_location.join("files"))?;
    }
    env::set_var("th", trash_home);
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
            confirm: String::from("yes"),
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
    let vec_target_abs = conv_to_abs(args);
    let (files, dirs) = is_file_or_dir(vec_target_abs);
    let user_args = UserCommand::new(files, dirs, f, c, z);
    println!("{:?}", user_args);
    if user_args.z {
        regret();
    } else if !user_args.files.is_empty() | !user_args.dirs.is_empty() {
        let path_log: PathBuf = PathBuf::from(env::var("th").unwrap()).join("log");
        let file_log = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path_log)
            .unwrap();
        if user_args.f {
            move_forever(user_args.files, file_log);
        } else {
            move_to_trash(user_args.files, &file_log, "file");
            move_to_trash(user_args.dirs, &file_log, "directory");
        }
    } else if user_args.c {
        clear();
    }
}
fn conv_to_abs(src: Vec<&str>) -> Vec<PathBuf> {
    let mut rst: Vec<PathBuf> = Vec::new();
    for s in src {
        let t: PathBuf = PathBuf::from(s).absolutize().unwrap().into_owned();
        rst.push(t);
    }
    rst
}
fn is_file_or_dir<P>(path: Vec<P>) -> (Vec<P>, Vec<P>)
where
    P: AsRef<Path>,
{
    let mut dirs: Vec<P> = Vec::new();
    let mut files: Vec<P> = Vec::new();
    for p in path {
        if Path::is_dir(p.as_ref()) {
            dirs.push(p);
        } else if Path::is_file(p.as_ref()) {
            files.push(p);
        } else {
            println!("{} not a file or a directory", p.as_ref().display());
        }
    }
    (files, dirs)
}
fn move_to_trash(targets: Vec<PathBuf>, mut log: &File, ident: &str) {
    let to: PathBuf = PathBuf::from(env::var("th").unwrap()).join("files");
    let time_local = Dime::OffsetDateTime::now_utc()
        .to_offset(offset!(+8))
        .format(
            &format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory]:[offset_minute]:[offset_second]",
            )
            .unwrap(),
        )
        .unwrap();
    #[allow(unused_assignments)]
    let mut log_info = String::new();
    let user: String = env::var("USER").unwrap();
    for target in targets {
        if ident.eq("directory") && env::var("PWD").unwrap().starts_with(target.to_str().unwrap()) {
            log_info = format!("{} {} tried to delete directory \"{}\" while I refused: Forbid to delete ancestor\n", time_local, &user, target.display());
            log.write_all(log_info.as_bytes()).unwrap();
            continue;
        }
        match rename(&target, to.join(target.file_name().unwrap())) {
            Ok(_) => {
                log_info = format!(
                    "{} {} deleted {} \"{}\"\n",
                    time_local,
                    &user,
                    ident,
                    target.display()
                );
            }
            Err(e) => {
                log_info = format!(
                    "{} {} tried to delete {} \"{}\" while an error occured: {} \n",
                    time_local,
                    &user,
                    ident,
                    target.display(),
                    e
                );
                eprintln!("{}", log_info);
            }
        }
        log.write_all(log_info.as_bytes()).unwrap();
    }
}
fn move_forever(files: Vec<PathBuf>, log: File) {
    for file in files {
        // let file_path_abs = Path::new(file).absolutize().unwrap();
        remove_file(file).unwrap();
    }
}
fn log<P>(abs_path: P)
where
    P: AsRef<Path>,
{
}
fn regret() {
    todo!()
}
fn clear() {
    todo!()
}
