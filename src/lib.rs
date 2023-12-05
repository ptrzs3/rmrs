extern crate libc;
use libc::time_t;
use libc::{chmod, mode_t};
use libc::{utimbuf, utime};
use std::ffi::CString;
use std::io;
use std::{
    env,
    fs::{self, File},
    io::{stdin, Read, Write, stdout},
    path::{Path, PathBuf},
};
pub mod unify;
pub mod error;
use error::AppError;
use path_absolutize::Absolutize;
use regex::Regex;
use serde::{Deserialize, Serialize};
const CONFIG_FILE: &'static str = ".rmrs.toml";

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub location: String,
    pub need_confirm_again: bool,
}
/// A struct to store args
#[derive(Debug)]
pub struct UserCommand<T>
where
    T: AsRef<Path>,
{
    pub targets: Vec<T>,
    pub f: bool,
    pub c: bool,
    pub z: bool,
    pub b: bool,
}

impl<T> UserCommand<T>
where
    T: AsRef<Path>,
{
    pub fn new(files: Vec<T>, f: bool, c: bool, z: bool, b: bool) -> UserCommand<T> {
        Self {
            targets: files,
            f,
            c,
            z,
            b,
        }
    }
}
pub fn update_file_mtime(file_path: &str, new_mtime: time_t) -> Result<(), String> {
    let path = CString::new(file_path).map_err(|_| "CString::new failed")?;
    let times = utimbuf {
        actime: new_mtime,   // 访问时间
        modtime: new_mtime,  // 修改时间
    };

    let ret = unsafe { utime(path.as_ptr(), &times) };
    if ret == 0 {
        Ok(())
    } else {
        Err("utime failed".to_string())
    }
}

pub fn change_file_permissions(file_path: &str, mode: mode_t) -> Result<(), String> {
    let c_file_path = CString::new(file_path).map_err(|_| "Failed to create CString")?;

    let result = unsafe { chmod(c_file_path.as_ptr(), mode) };
    if result == 0 {
        Ok(())
    } else {
        Err("Failed to change file permissions".to_string())
    }
}

pub fn get_dir_size(pb: &PathBuf) -> io::Result<u64> {
    let mut dir_size: u64 = 0;
    for p in fs::read_dir(pb)? {
        let pt = p?.path();
        if pt.is_file() {
            dir_size = dir_size + pt.metadata()?.len();
        } else if pt.is_dir() {
            dir_size = dir_size + get_dir_size(&pt)?;
        }
    }
    Ok(dir_size)
}

pub fn friendly_size(size: u64) -> String {
    let units: Vec<&str> = vec!["Bytes", "KB", "MB", "GB", "TB", "PB"];
    let mut ptr: usize = 0;
    let mut fsize: f64 = size as f64;
    while fsize >= 1000.00 {
        fsize = fsize / 1000.00;
        ptr = ptr + 1;
    }
    format!("{:.2} {}", fsize, units[ptr])
}

pub fn confirm() -> bool {
    if env::var("ca").unwrap().eq("false") {
        return true;
    }
    print!("Are you sure? [Y/n] ");
    stdout().flush().unwrap();
    let mut s: String = String::new();
    stdin().read_line(&mut s).unwrap();
    return s.eq("Y\n");
}
pub fn check_exist(f: String) -> Result<String, AppError> {
    let to: PathBuf = PathBuf::from(env::var("tc").unwrap()).join(&f);
    let tc: PathBuf = PathBuf::from(env::var("tc").unwrap());
    if to.exists() {
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
fn update_file_name(ori: &str, idx: usize, i: &u16) -> String {
    let mut dst: String = String::from(ori);
    dst.insert_str(idx, format!("{}", i).as_str());
    dst
}
fn prefix(f: &str) -> usize {
    for (idx, c) in f.chars().enumerate() {
        if idx != 0 && c == '.' {
            return idx;
        }
    }
    f.len()
}
pub fn conv_to_abs(src: Vec<&str>) -> Vec<PathBuf> {
    let mut abs: Vec<PathBuf> = Vec::new();
    for s in src {
        let t: PathBuf = PathBuf::from(s).absolutize().unwrap().into_owned();
        abs.push(t);
    }
    abs
}
pub fn is_valid_path(p: &str) -> bool {
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
pub fn get_type(t: &PathBuf) -> String {
    if t.is_dir() {
        return "directory".to_string();
    } else if t.is_file(){
        return "file".to_string();   
    } else {
        return "undefined type".to_string();
    }
}

pub fn proc_toml() -> Result<(String, bool), AppError> {
    let mut p = PathBuf::new();
    let envv = unify::ENVV {
        #[cfg(not(target_os="windows"))]
        home:  String::from("HOME"),
        #[cfg(target_os="windows")]
        home: String::from("HOMEPATH"),
    };
    p.push(match env::var(&envv.home) {
        Ok(v) => v,
        Err(e) => {
            return Err(e.into());
        }
    });
    p.push(CONFIG_FILE);
    if Path::exists(&p) {
        let mut conf = File::open(&p)?;
        let mut content = String::new();
        conf.read_to_string(&mut content)?;
        if let Ok(config) = toml::from_str::<Config>(&content) {
            return Ok((config.location, config.need_confirm_again));
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
            "\tLooks like you haven't used rmrs yet\n\
            \tThe default trash location would be \"{}/.rtrash\"\n\
        \tor you may input customized trash location(absolute):",
            env::var(&envv.home).unwrap()
        );
        let mut user_input: String = String::new();
        stdin().read_line(&mut user_input)?;
        user_input.pop();
        let mut config: Config = Config {
            location: String::from(""),
            need_confirm_again: true,
        };
        if is_valid_path(&user_input) {
            config.location = user_input;
        } else {
            config.location = format!("{}/.rtrash", env::var(&envv.home).unwrap());
        }
        let content = toml::to_string(&config)?;
        conf.write_all(content.as_bytes())?;
        conf.flush()?;
        return Ok((config.location, config.need_confirm_again));
    }
}
