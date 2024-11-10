use event::Event;
use serde::{ Serialize, Deserialize };
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
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

    let mut async_pool= async_pool::AsyncPool::new(10, Duration::from_millis(1));
    
    async_pool.connect_listener(|e| {
        match e.as_ref() {
            Event::Log(str) => { println!("{}", str)}
            Event::Warn(str) => { println!("{}", str)}
            Event::Error(str) => { println!("{}", str)}
        }
    });

    let async_pool = Rc::new(RefCell::new(async_pool));


    loop {
        sleep(Duration::from_secs(1));
    }
}
