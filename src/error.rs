use std::{env::VarError, fmt::Display, io};
use time as Dime;
#[derive(Debug)]
pub struct AppError {
    pub code: i8,
    pub message: String,
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
