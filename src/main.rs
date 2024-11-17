use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use std::env;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::{fs, time::Duration};

mod async_pool;
mod control;
mod error;
mod event;
mod signal;
mod source;

use control::Config;
use control::Control;
use event::Event;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MainConfig {
    thread_count: Option<usize>,
    timer_resolution: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FileConfig {
    control: Vec<Config>,
    main: Option<MainConfig>,
    // possibly add the option for dynamic threads
}

fn spawn_notification(message: &str) {
    let _ = Notification::new()
        .summary("Kool Error")
        .body(message)
        .appname("kool")
        .show();
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Argument for config path file missing");
        exit(1);
    }

    let path_to_config = &args[1];

    println!("Loading config {path_to_config}");

    let tomls = fs::read_to_string(path_to_config).expect("Cannot read from file");

    let config: FileConfig = toml::from_str(&tomls).expect("Cannot parse");

    println!("{:#?}", config);

    let mut thread_count = 10;
    let mut timer_resolution = 1;

    if let Some(main) = config.main {
        if let Some(th) = main.thread_count {
            thread_count = th;
        }

        if let Some(res) = main.timer_resolution {
            timer_resolution = res;
        }
    }

    let mut async_pool =
        async_pool::AsyncPool::new(thread_count, Duration::from_millis(timer_resolution));

    async_pool.connect_listener(|e| match e.as_ref() {
        Event::Log(str) => {
            println!("Log: {}", str);
        }
        Event::Warn(str) => {
            println!("Warn: {}", str);
        }
        Event::Error(err) => {
            println!("Error: {}", err.message());
            spawn_notification(err.message().as_str());
        }
        Event::LogError(str) => {
            println!("LogError: {}", str);
        }
    });

    for control_config in config.control {
        match Control::new(control_config) {
            Ok(control) => {
                let interval = control.get_interval().clone();
                let control = Arc::new(Mutex::new(control));

                async_pool.attach_job(interval, move || {
                    if let Err(e) = control.lock().unwrap().control() {
                        Some(e)
                    } else {
                        None
                    }
                });
            }
            Err(err) => {
                spawn_notification(err.message().as_str());
            }
        }
    }

    async_pool.wait();
}
