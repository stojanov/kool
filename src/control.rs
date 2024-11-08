use serde::{ Serialize, Deserialize };
use std::fs;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Config {
    name: String,
    interval: i64,
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

trait Source {
    fn get(self: Box<Self>) -> i64;
}

pub struct Control {
    config: Config,
    source: Box<dyn Source>,
    // for now we only support files
    dest: fs::File,
}

impl Control {
    fn new(conf: Config) -> Self {
    }
}
