use clap::{self, command, Arg, ArgAction};
use regex::Regex;
use rmrs::{check_exist, confirm};
use rmrs::{conv_to_abs, error::AppError, get_type, proc_toml, UserCommand};
use std::{
    env::{self},
    fs::{self, remove_dir_all, remove_file, rename, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
use time as Dime;
use Dime::{format_description, macros::offset};

#[allow(unused_assignments)]
fn main() -> Result<(), AppError> {
    let mut trash_home = String::new();
    let mut confirm_again = true;
    (trash_home, confirm_again)  = proc_toml()?;
    let trash_can = Path::new(trash_home.as_str()).join("files");
    if !trash_can.exists() {
        fs::create_dir_all(&trash_can)?;
    }
    env::set_var("th", trash_home);
    env::set_var("tc", trash_can);
    env::set_var("ca", confirm_again.to_string());
    run()
}

fn run() -> Result<(), AppError> {
    let matches = command!()
        .about("A rm-like tool written in rust.")
        .author("ptrzs3 https://github.com/ptrzs3")
        .help_template(
            "\
{about-with-newline}
version: {version}\n
author: {author-with-newline}
{usage-heading} {usage}

{all-args}
        ",
        )
        .arg(
            Arg::new("targets")
                .action(ArgAction::Append)
                .required(false)
                .help("file(s) or dir(s) or both of them you want to delete"),
        )
        // .arg(
        //     Arg::new("location")
        //         .action(ArgAction::Set)
        //         .required(false)
        //         .short('l')
        //         .long("location")
        //         .help("set trash location"),
        // )
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
                .long("undo")
                .help("Undo the last operation if trash hasn't been cleared"),
        )
        .get_matches();
    let args = matches
        .get_many::<String>("targets")
        .unwrap_or_default()
        .map(|v| v.as_str())
        .collect::<Vec<_>>();
    let f = matches.get_flag("forever");
    let c = matches.get_flag("clear");
    let z = matches.get_flag("regret");
    let vec_target_abs = conv_to_abs(args);
    let user_args = UserCommand::new(vec_target_abs, f, c, z);
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
    if user_args.z {
        regret(&file_log, &time_local)
    } else if !user_args.targets.is_empty() {
        if user_args.f {
            move_to_trash(user_args.targets, &file_log, &time_local, true)
        } else {
            move_to_trash(user_args.targets, &file_log, &time_local, false)
        }
    } else if user_args.c {
        clear(&file_log, &time_local)
    } else {
        Ok(())
    }
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
            let fty = get_type(&target);
            match remove_file(&target) {
                Ok(_) => {
                    info_log = format!(
                        "{} {} permanently deleted {} \"{}\"\n",
                        now,
                        &user,
                        fty,
                        target.display(),
                    );
                }
                Err(e) => {
                    info_log = format!(
                        "{} {} tried to permanently delete {} \"{}\" while an error occured: {} \n",
                        now,
                        &user,
                        fty,
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
                    let fty = get_type(&target);
                    match rename(&target, to.join(&n)) {
                        Ok(_) => {
                            info_log = format!(
                                "{} {} deleted {} \"{}\" => {}\n",
                                now,
                                &user,
                                // get_type(&target),
                                fty,
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
                                fty,
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

fn regret(mut log: &File, now: &str) -> Result<(), AppError> {
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
        #[allow(unused_assignments)]
        let mut log_info: String = String::new();
        match fs::rename(&v[0], &v[1]) {
            Ok(_) => {
                log_info = format!(
                    "{} {} undid last opeation successfully\n",
                    now,
                    env::var("USER").unwrap_or("default".to_string())
                );
                log.write_all(log_info.as_bytes())?;
            }
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        log_info = format!(
                            "{} {} tried to undo last operation while an error occured: {}\n",
                            now,
                            env::var("USER").unwrap_or("default".to_string()),
                            e.kind().to_string()
                        );
                        log.write_all(log_info.as_bytes())?;
                        // 发生NotFound错误，删除.last文件
                        remove_file(&path_last)?;
                    }
                    _ => {
                        log_info = format!(
                            "{} {} tried to undo last operation while an error occured: {}\n",
                            now,
                            env::var("USER").unwrap_or("default".to_string()),
                            e.kind().to_string()
                        );
                        log.write_all(log_info.as_bytes())?;
                    }
                }
                return Err(e.into());
            }
        }
    }
    // 全部成功恢复，删除.last文件
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
