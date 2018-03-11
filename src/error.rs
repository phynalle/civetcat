use std::error::{self, Error as Err};
use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    UnsupportedTheme,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::UnsupportedTheme => "Unsupported Theme",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => e.fmt(f),
            _ => write!(f, "{}", self.description()),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
