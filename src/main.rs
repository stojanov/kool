use event::Event;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::{env, path, thread};
use std::{fs, io::Write, thread::sleep, time::Duration};

mod async_pool;
mod control;
mod event;
mod signal;

use control::control::Config;
use control::control::Control;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FileConfig {
    control: Vec<Config>,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 1 {
        println!("Path file missing");
    }

    let path_to_config = &args[1];

    println!("Loading config {path_to_config}");

    let tomls = fs::read_to_string(path_to_config).expect("Cannot read from file");

    let config: FileConfig = toml::from_str(&tomls).expect("Cannot parse");

    println!("{:#?}", config);

    let mut async_pool = async_pool::AsyncPool::new(10, Duration::from_millis(1));

    async_pool.connect_listener(|e| match e.as_ref() {
        Event::Log(str) => {
            println!("Log {}", str)
        }
        Event::Warn(str) => {
            println!("Warn {}", str)
        }
        Event::Error(str) => {
            println!("Error {}", str)
        }
    });

    let controls: Vec<Arc<Mutex<Control>>> = config
        .control
        .iter()
        .map(|config| Control::new(config.clone()))
        .filter(|control| if let Some(c) = control { true } else { false })
        .map(|control| Arc::new(Mutex::new(control.unwrap())))
        .collect();

    for control in controls.iter() {
        let interval = control.lock().unwrap().get_interval().clone();
        let c = Arc::clone(control);

        async_pool.attach_job(interval, move || {
            c.lock().unwrap().control();
        });
    }
    async_pool.wait();
}
