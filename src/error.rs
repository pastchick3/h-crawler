use reqwest::Error as ReqError;
use rusqlite::Error as SqlError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Error as IoError;
use std::num::ParseIntError;

pub struct DisplayableError(Box<dyn Display>);

impl Display for DisplayableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let DisplayableError(err) = self;
        write!(f, "{}", err)
    }
}

impl From<ReqError> for DisplayableError {
    fn from(error: ReqError) -> Self {
        DisplayableError(Box::new(error))
    }
}

impl From<IoError> for DisplayableError {
    fn from(error: IoError) -> Self {
        DisplayableError(Box::new(error))
    }
}

impl From<String> for DisplayableError {
    fn from(error: String) -> Self {
        DisplayableError(Box::new(error))
    }
}

impl From<&'static str> for DisplayableError {
    fn from(error: &'static str) -> Self {
        DisplayableError(Box::new(error))
    }
}

impl From<ParseIntError> for DisplayableError {
    fn from(error: ParseIntError) -> Self {
        DisplayableError(Box::new(error))
    }
}

impl From<SqlError> for DisplayableError {
    fn from(error: SqlError) -> Self {
        DisplayableError(Box::new(error))
    }
}
