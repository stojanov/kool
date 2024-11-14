use crate::error;

pub enum Event {
    Log(String),
    Warn(String),
    Error(error::Error),
    LogError(String),
}
