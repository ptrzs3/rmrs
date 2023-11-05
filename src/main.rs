use clap::{self, command, Arg, ArgAction};
use path_absolutize::Absolutize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    env::{self, VarError},
    fmt::Display,
    fs::{self, remove_dir_all, remove_file, rename, File, OpenOptions},
    io::{self, stdin, BufRead, BufReader, Read, Write},
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
    targets: Vec<T>,
    f: bool,
    c: bool,
    z: bool,
}

impl<T> UserCommand<T>
where
    T: AsRef<Path>,
{
    fn new(files: Vec<T>, f: bool, c: bool, z: bool) -> UserCommand<T> {
        Self {
            targets: files,
            f,
            c,
            z,
        }
    }
}
#[derive(Debug)]
struct AppError {
    code: i8,
    message: String,
}
impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "code: {}, message: {}", self.code, self.message)
    }
}
impl From<toml::ser::Error> for AppError {
    fn from(value: toml::ser::Error) -> Self {
        Self {
            code: -6,
            message: value.to_string(),
        }
    }
}
impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        let code = match value.kind() {
            io::ErrorKind::NotFound => -1_i8,
            io::ErrorKind::PermissionDenied => -2_i8,
            _ => 0x0f_i8,
        };
        Self {
            code,
            message: value.kind().to_string(),
        }
    }
}
impl From<VarError> for AppError {
    fn from(value: VarError) -> Self {
        let code = match value {
            VarError::NotPresent => -4_i8,
            VarError::NotUnicode(_) => -5_i8,
        };
        Self {
            code,
            message: value.to_string(),
        }
    }
}
impl From<Dime::error::Format> for AppError {
    fn from(value: Dime::error::Format) -> Self {
        Self {
            code: -7,
            message: value.to_string(),
        }
    }
}
impl From<Dime::error::InvalidFormatDescription> for AppError {
    fn from(value: Dime::error::InvalidFormatDescription) -> Self {
        Self {
            code: -8,
            message: value.to_string(),
        }
    }
}
impl From<regex::Error> for AppError {
    fn from(value: regex::Error) -> Self {
        Self {
            code: -9,
            message: value.to_string(),
        }
    }
}
#[allow(unused_assignments)]
fn main() -> Result<(), AppError> {
    let mut trash_home = String::new();
    if let Ok(location) = env::var("TRASH_HOME") {
        trash_home = location;
    } else {
        trash_home = proc_toml()?;
    }
    let trash_can = Path::new(trash_home.as_str()).join("files");
    if !trash_can.exists() {
        fs::create_dir_all(&trash_can)?;
    }
    env::set_var("th", trash_home);
    env::set_var("tc", trash_can);
    run()
}

fn proc_toml() -> Result<String, AppError> {
    let mut p = PathBuf::new();
    p.push(match env::var("HOME") {
        Ok(v) => v,
        Err(e) => {
            return Err(e.into());
        }
    });
    p.push(CONFIG_FILE);
    // toml exists
    if Path::exists(&p) {
        let mut conf = File::open(&p)?;
        let mut content = String::new();
        conf.read_to_string(&mut content)?;
        if let Ok(config) = toml::from_str::<Config>(&content) {
            return Ok(config.location);
        } else {
            fs::remove_file(&p)?;
            Err(AppError {
                code: -3_i8,
                message: format!("broken \"{}\", please rerun", p.to_string_lossy()),
            })
        }
    } else {
        let mut conf = File::create(&p)?;
        // 前面已经处理过Result(env::var)，这里可以放心unwrap
        println!(
            "\t\tLooks like you haven't used rmrs yet\n\
            \t\tThe default trash location would be \"{}/.trash\"\n\
        \t\tor you may input customized trash location(absolute):",
            env::var("HOME").unwrap()
        );
        let mut user_input: String = String::new();
        stdin().read_line(&mut user_input)?;
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
        let content = toml::to_string(&config)?;
        conf.write_all(content.as_bytes())?;
        conf.flush()?;
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
fn run() -> Result<(), AppError> {
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
    // let (files, dirs) = is_file_or_dir(vec_target_abs);
    let user_args = UserCommand::new(vec_target_abs, f, c, z);
    println!("{:?}", user_args);
    let path_log: PathBuf = PathBuf::from(env::var("th").unwrap()).join("log");
    let file_log = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path_log)?;

    let time_local = Dime::OffsetDateTime::now_utc()
        .to_offset(offset!(+8))
        .format(&format_description::parse(
            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory]:[offset_minute]:[offset_second]",
        )?)?;
    // .unwrap();
    if user_args.z {
        regret()
    } else if !user_args.targets.is_empty() {
        if user_args.f {
            move_to_trash(user_args.targets, &file_log, &time_local, true)
        } else {
            move_to_trash(user_args.targets, &file_log, &time_local, false)
        }
    } else if user_args.c {
        clear(&file_log, &time_local)
    } else {
        Err(AppError {
            code: 0,
            message: "Bye~".to_string(),
        })
    }
}

/// convert relative(or absolute) path to absoulute path
fn conv_to_abs(src: Vec<&str>) -> Vec<PathBuf> {
    let mut abs: Vec<PathBuf> = Vec::new();
    for s in src {
        let t: PathBuf = PathBuf::from(s).absolutize().unwrap().into_owned();
        abs.push(t);
    }
    abs
}

#[allow(dead_code)]
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
fn move_to_trash(
    targets: Vec<PathBuf>,
    mut log: &File,
    now: &str,
    permanently: bool,
) -> Result<(), AppError> {
    let path_last = PathBuf::from(env::var("th").unwrap()).join(".last");
    let mut file_last = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path_last)?;
    let to: PathBuf = PathBuf::from(env::var("tc").unwrap());
    #[allow(unused_assignments)]
    let mut info_log = String::new();
    #[allow(unused_assignments)]
    let mut info_last = String::new();
    let user: String = env::var("USER").unwrap_or("default".to_string());
    for target in targets {
        if target.is_dir() && env::var("PWD")?.starts_with(target.to_str().unwrap()) {
            info_log = format!("{} {} tried to delete directory \"{}\" while I refused: Forbid to delete ancestor\n", now, &user, target.display());
            log.write_all(info_log.as_bytes())?;
            continue;
        }
        if permanently {
            match remove_file(&target) {
                Ok(_) => {
                    info_log = format!(
                        "{} {} permanently deleted {} \"{}\"\n",
                        now,
                        &user,
                        get_type(&target),
                        target.display(),
                    );
                }
                Err(e) => {
                    info_log = format!(
                        "{} {} tried to permanently delete {} \"{}\" while an error occured: {} \n",
                        now,
                        &user,
                        get_type(&target),
                        target.display(),
                        e
                    );
                    eprintln!("{e}");
                }
            }
            log.write_all(info_log.as_bytes())?;
        } else {
            match check_exist(target.file_name().unwrap().to_string_lossy().into_owned()) {
                Ok(n) => {
                    match rename(&target, to.join(&n)) {
                        Ok(_) => {
                            info_log = format!(
                                "{} {} deleted {} \"{}\" => {}\n",
                                now,
                                &user,
                                get_type(&target),
                                target.display(),
                                &n
                            );
                            info_last = format!(
                                "{} >> {}\n",
                                PathBuf::from(env::var("tc").unwrap()).join(&n).display(),
                                target.display()
                            );
                            file_last.write_all(info_last.as_bytes())?;
                        }
                        Err(e) => {
                            info_log = format!(
                                "{} {} tried to delete {} \"{}\" while an error occured: {} \n",
                                now,
                                &user,
                                get_type(&target),
                                target.display(),
                                e
                            );
                            eprintln!("{e}");
                        }
                    }
                    log.write_all(info_log.as_bytes())?;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
    Ok(())
}
fn get_type(t: &PathBuf) -> String {
    if t.is_dir() {
        return "directory".to_string();
    }
    return "file".to_string();
}
fn prefix(f: &str) -> usize {
    for (idx, c) in f.chars().enumerate() {
        if idx != 0 && c == '.' {
            return idx;
        }
    }
    f.len()
}
fn update_file_name(ori: &str, idx: usize, i: &u16) -> String {
    let mut dst: String = String::from(ori);
    dst.insert_str(idx, format!("{}", i).as_str());
    dst
}
fn check_exist(f: String) -> Result<String, AppError> {
    let to: PathBuf = PathBuf::from(env::var("tc").unwrap()).join(&f);
    let tc: PathBuf = PathBuf::from(env::var("tc").unwrap());
    if to.exists() {
        println!("exist");
        let idx = prefix(&f);
        if let Some(new_name) = (2_u16..)
            .map(|i| update_file_name(&f, idx, &i))
            .find(|n| !tc.join(n).exists())
        {
            return Ok(new_name);
        } else {
            return Err(AppError {
                code: -10,
                message: "number exceed u16".to_string(),
            });
        }
    }
    Ok(f)
}
fn regret() -> Result<(), AppError> {
    let path_last = PathBuf::from(env::var("th").unwrap()).join(".last");
    let f = File::open(&path_last)?;
    let mut reader = BufReader::new(f);
    let mut lines: Vec<String> = vec![];
    let mut line = String::new();
    line.clear();
    let mut len = reader.read_line(&mut line)?;
    let re = Regex::new(r"[/+.?_?\-?\w?]+")?;
    while len > 0 {
        line.pop();
        lines.push(line.clone());
        line.clear();
        len = reader.read_line(&mut line)?;
    }
    for li in lines {
        let mut v: Vec<String> = Vec::new();
        for cap in re.captures_iter(&li) {
            v.push(cap[0].to_string());
        }
        // println!("from: {}, to: {}", v[0], v[1]);
        fs::rename(&v[0], &v[1])?;
    }
    remove_file(&path_last)?;
    Ok(())
}
fn clear(mut log: &File, now: &str) -> Result<(), AppError> {
    if confirm() {
        #[allow(unused_assignments)]
        let mut log_info: String = String::new();
        match remove_dir_all(PathBuf::from(env::var("th").unwrap()).join("files")) {
            Ok(_) => {
                log_info = format!(
                    "{} {} cleaned trash can\n",
                    now,
                    env::var("USER").unwrap_or("default".to_string())
                );
            }
            Err(e) => {
                log_info = format!(
                    "{} {} tried to clean trash can while an error occured: {}",
                    now,
                    env::var("USER").unwrap_or("default".to_string()),
                    e
                );
                eprintln!("{e}");
            }
        }
        log.write_all(log_info.as_bytes())?;
    }
    Ok(())
}
fn confirm() -> bool {
    println!("Are you sure: (Y/N)?");
    let mut s: String = String::new();
    stdin().read_line(&mut s).unwrap();
    s.pop();
    return s.eq("Y");
}
