use event::Event;
use serde::{ Serialize, Deserialize };
use std::{fs, io::Write, thread::sleep, time::Duration};

mod async_pool;
mod signal;
mod event;
mod control;

use control::control::Config;

#[derive(Debug, Serialize, Deserialize)]
struct FileConfig {
    control: Vec<Config>
}

fn main() {
    let tomls = fs::read_to_string("example.toml").expect("Cannot read from file");

    let parsed: FileConfig = toml::from_str(&tomls).expect("Cannot parse");

    println!("{:#?}", parsed);

    let mut input = String::new();

    let mut p = async_pool::AsyncPool::new(10, Duration::from_millis(1));
    
    p.connect_listener(|e| {
        match e.as_ref() {
            Event::Log(str) => { println!("{}", str)}
            Event::Warn(str) => { println!("{}", str)}
            Event::Error(str) => { println!("{}", str)}
        }
    });

    p.attach_job(Duration::from_millis(500), || {
        println!("From thread");
    });

    p.attach_job(Duration::from_millis(100), || {
        println!("From thread but faster");
    });

    loop {
        sleep(Duration::from_secs(1));
    }
}
