use serde::{ Serialize, Deserialize };
use std::{fs::{self, OpenOptions}, time::Duration};

use super::source;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Config {
    name: String,
    interval: u64,
    src_path: String,
    src_type: String,
    src_args: Option<Vec<String>>,
    dest_path: String,
    dest_min: i64,
    dest_max: i64,
    default_dest_percent: Option<i32>,
    curve: Option<String>,
    points: Vec<Vec<i64>>,
}

pub struct Control {
    config: Config,
    source: Box<dyn source::Source>,
    // for now only file is supported
    dest: fs::File,
}

impl Control {
    fn new(config: Config) -> Option<Self> {
        let source: Box<dyn source::Source>;

        let timeout = Duration::from_millis(config.interval);

        match config.src_type.to_lowercase().as_str() {
            "file" => {
                let src = source::FileSource::new(&config.src_path, timeout);

                if let Some(src) = src {
                    source = Box::new(src);
                } else {
                    return None;
                }
            },
            "program" => {
                if let Some(&args) = config.src_args {
                    let src = source::ProgramSource::new(&config.src_path, args);

                    if let Some(src) = src {
                        source = Box::new(src);
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }

            },
            _ => {
                return None;
            }
        }

        let dest = OpenOptions::new()
            .write(true)
            .open(&config.dest_path);

        if let Err(_) = dest {
            return None
        }

        Some(Self{
            config,
            source,
            dest: dest.unwrap()
        })
    }
}
