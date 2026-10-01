use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Syntax {
        file: String,
        line: u32,
        msg: String,
    },
    Io {
        path: String,
        err: std::io::Error,
    },
}

impl Error {
    pub fn syntax(file: &str, line: u32, msg: impl Into<String>) -> Self {
        Error::Syntax {
            file: file.to_string(),
            line,
            msg: msg.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax { file, line, msg } => write!(f, "{file}:{line}: {msg}"),
            Error::Io { path, err } => write!(f, "{path}: {err}"),
        }
    }
}

impl std::error::Error for Error {}
