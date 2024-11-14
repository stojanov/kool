use std::{
    fs::{self, OpenOptions},
    io::Read,
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};

use crate::error::{self, Error};

pub trait Source {
    fn get(&mut self, timeout: Duration) -> Result<i64, Error>;
}

pub struct ProgramSource {
    command: Command,
}

impl ProgramSource {
    pub fn new(path: &String, args: Option<&Vec<String>>) -> Self {
        let mut command = Command::new(path);

        if let Some(args) = args {
            command.args(args);
        }

        command.stdout(Stdio::piped());

        Self { command }
    }
}

impl Source for ProgramSource {
    fn get(&mut self, timeout: Duration) -> Result<i64, Error> {
        match self.command.spawn() {
            Ok(mut child) => {
                sleep(timeout);

                if let Ok(Some(status)) = child.try_wait() {
                    if status.code().unwrap() == 0 {
                        if let Some(mut stdout) = child.stdout {
                            let mut result = String::new();

                            stdout.read_to_string(&mut result);

                            match result.parse::<i64>() {
                                Ok(n) => Ok(n),
                                Err(err) => Err(Error::new(
                                    error::Code::UnableToParse,
                                    String::from("Unable to parse response from command"),
                                )),
                            }
                        } else {
                            Err(Error::new(
                                error::Code::General,
                                String::from("Unable to capture stdout for the command"),
                            ))
                        }
                    } else {
                        Err(Error::new(
                            error::Code::General,
                            String::from("Command didn't exit successfuly"),
                        ))
                    }
                } else {
                    child.kill();
                    Err(Error::new(
                        error::Code::Timeout,
                        String::from("Command timedout"),
                    ))
                }
            }
            Err(_) => Err(Error::new(
                error::Code::UnableToSpawnCommand,
                String::from("Unable to spawn a command"),
            )),
        }
    }
}

pub struct FileSource {
    file: fs::File,
}

impl FileSource {
    pub fn new(path: &String) -> Option<Self> {
        let file = OpenOptions::new().read(true).open(path);

        match file {
            Ok(file) => Some(Self { file }),
            Err(_) => None,
        }
    }
}

impl Source for FileSource {
    fn get(&mut self, _timeout: Duration) -> Result<i64, Error> {
        let mut buffer = String::new();

        self.file.read_to_string(&mut buffer);

        match buffer.parse::<i64>() {
            Ok(n) => Ok(n),
            Err(_) => Err(Error::new(
                error::Code::UnableToParse,
                String::from("Unable to parse read information from file"),
            )),
        }
    }
}
