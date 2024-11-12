use std::{
    fs::{self, OpenOptions},
    io::Read,
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};

pub trait Source {
    fn get(&mut self, timeout: Duration) -> Option<i64>;
}

pub struct ProgramSource {
    command: Command,
}

impl ProgramSource {
    pub fn new(path: &String, args: &Vec<String>) -> Option<Self> {
        let mut command = Command::new(path);

        command.args(args);
        command.stdout(Stdio::piped());

        Some(Self { command })
    }
}

impl Source for ProgramSource {
    fn get(&mut self, timeout: Duration) -> Option<i64> {
        match self.command.spawn() {
            Ok(mut child) => {
                sleep(timeout);

                if let Ok(Some(status)) = child.try_wait() {
                    if status.code().unwrap() == 0 {
                        let mut result = String::new();

                        let _ = child.stdout.take()?.read_to_string(&mut result);

                        return match result.parse::<i64>() {
                            Ok(n) => Some(n),
                            Err(_) => None,
                        };
                    }

                    None
                } else {
                    let _ = child.kill();
                    None
                }
            }
            Err(_) => return None,
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
    fn get(&mut self, timeout: Duration) -> Option<i64> {
        let mut buffer = String::new();

        self.file.read_to_string(&mut buffer);

        match buffer.parse::<i64>() {
            Ok(n) => Some(n),
            Err(_) => None,
        }
    }
}
