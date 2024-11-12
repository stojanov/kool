use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use toml::to_string;

use super::source;

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
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
    interval: Duration,
}

unsafe impl Send for Control {}
unsafe impl Sync for Control {}

impl Control {
    pub fn new(config: Config) -> Option<Self> {
        let source: Box<dyn source::Source>;

        let interval = Duration::from_millis(config.interval);

        match config.src_type.to_lowercase().as_str() {
            "file" => {
                let src = source::FileSource::new(&config.src_path);

                if let Some(src) = src {
                    source = Box::new(src);
                } else {
                    return None;
                }
            }
            "program" => {
                if let Some(args) = &config.src_args {
                    let src = source::ProgramSource::new(&config.src_path, args);

                    if let Some(src) = src {
                        source = Box::new(src);
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => {
                return None;
            }
        }

        let dest = OpenOptions::new().write(true).open(&config.dest_path);

        if let Err(_) = dest {
            println!("Cannot open file ");
            //return None;
        }

        Some(Self {
            config,
            source,
            dest: dest.unwrap(),
            interval,
        })
    }

    pub fn get_interval(&self) -> &Duration {
        &self.interval
    }

    fn lerp(in_min: i64, in_max: i64, in_current: i64, out_min: i64, out_max: i64) -> f64 {
        let in_range = in_max - in_min;
        let in_cur = in_current - in_min;

        let percent = in_cur as f64 / in_range as f64;

        (out_max - out_min) as f64 * percent + out_min as f64
    }

    pub fn control(&mut self) {
        let interval = self.config.interval as f64 * 0.5;

        let src = self.source.get(Duration::from_millis(interval as u64));

        if let Some(input) = src {
            let lower_idx = self.config.points.iter().position(|v| v[0] >= input);

            // take care of lower values
            if let Some(lidx) = lower_idx {
                if lidx >= (self.config.points.len() - 1) {
                    let point = self.config.points.get(lidx).unwrap();
                    self.write_pwm_raw(point[1]);
                } else {
                    let lower_point = self.config.points.get(lidx).unwrap();
                    let upper_point = self.config.points.get(lidx + 1).unwrap();

                    self.write_pwm(
                        lower_point[0],
                        upper_point[0],
                        input,
                        lower_point[1],
                        upper_point[1],
                    );
                }
            } else {
                if let Some(point) = self.config.points.first() {
                    self.write_pwm_raw(point[1]);
                }
            }
        }
    }

    fn write_pwm(&mut self, in_min: i64, in_max: i64, in_current: i64, out_min: i64, out_max: i64) {
        let raw_percent = Control::lerp(in_min, in_max, in_current, out_min, out_max);

        let pwm = Control::lerp(
            0,
            100,
            raw_percent as i64,
            self.config.dest_min,
            self.config.dest_max,
        ) as i64;

        println!("PWM WRITTEN {}", pwm);

        if let Ok(_) = self.dest.write_all(pwm.to_string().as_bytes()) {
            let _ = self.dest.flush();
        } else {
            println!("Cannot write");
        }
    }

    fn write_pwm_raw(&mut self, pwm: i64) {
        println!("PWM WRITTEN RAW {}", pwm);

        if let Ok(_) = self.dest.write_all(pwm.to_string().as_bytes()) {
            let _ = self.dest.flush();
        } else {
            println!("Cannot write");
        }
    }
}
