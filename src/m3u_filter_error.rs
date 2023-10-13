use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[macro_export]
macro_rules! get_notify_message {
    ($errors:expr, $size:expr) => {
        if $errors.is_empty() {
            None
        } else {
            let text = $errors.iter()
                        .filter(|&err| err.kind == M3uFilterErrorKind::Notify)
                        .map(|err| err.message.as_str())
                        .collect::<Vec<&str>>()
                        .join("\n");
            if $size > 0 && text.len() > std::cmp::max($size-3, 3) {
                Some(format!("{}...", text.get(0..$size).unwrap()))
            } else {
                Some(text)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum M3uFilterErrorKind {
    // do not send with messaging
    Info,
    Notify, // send with messaging
}

#[derive(Debug)]
pub(crate) struct M3uFilterError {
    pub kind: M3uFilterErrorKind,
    pub message: String,

}

impl M3uFilterError {
    pub fn new(kind: M3uFilterErrorKind, message: String) -> M3uFilterError {
        M3uFilterError {
            kind,
            message,
        }
    }
}

impl Display for M3uFilterError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "M3uFilter error: {}", self.message)
    }
}

impl Error for M3uFilterError {}

