use std::fmt;

#[derive(Clone)]
pub enum Code {
    General = 0x0000,
    SourceTypeIsRequired,
    CannotOpenDestinationFile,
    Timeout,
    UnableToSpawnCommand,
    UnableToParse,
    UnableToWrite,
    InvalidConfigCurvePoints,
}

pub struct Error {
    code: Code,
    message: String,
}

impl Error {
    pub fn new(code: Code, message: String) -> Self {
        Self { code, message }
    }

    pub fn code(&self) -> Code {
        self.code.clone()
    }

    pub fn message(&self) -> &String {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ControlError {{ code:{}, message:{} }}",
            self.code.clone() as usize,
            self.message
        )
    }
}
