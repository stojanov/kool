use std::{error::Error, fs};


pub trait Source {
    fn get(&mut self) -> Result<i64, Box<dyn Error>>;
}

struct ProgramSource {
    path: String,
    args: Vec<String>
}

impl ProgramSource {
    fn new(path: &String, args: &Vec<String>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            path: path.clone(),
            args: args.clone()
        })
    }
}

impl Source for ProgramSource {
    fn get(&mut self) -> Result<i64, Box<dyn Error>> {
        self.path = String::from("Test");
        return Ok(1);
    }
}

struct FileSource {
    file: fs::File,
}

impl FileSource {
    fn new(path: &String) -> Result<Self, Box<dyn Error>> {
        let file = fs::OpenOptions::new()
            .read(true)
            .open(path)?;

        Ok(Self {
            file
        })
    }
}

