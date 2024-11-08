use serde::{ Serialize, Deserialize };
use std::{fs, io::Write};

mod control;
mod async_pool;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    control: Vec<control::Config>,
}

fn main() {
    let tomls = fs::read_to_string("example.toml").expect("Cannot read from file");

    let parsed: Config = toml::from_str(&tomls).expect("Cannot parse");

    println!("{:#?}", parsed);


    let mut pwm = fs::OpenOptions::new()
        .write(true)
        .open("/sys/class/hwmon/hwmon4/pwm4")
        .unwrap();

    use std::io;

    let mut input = String::new();

    loop {
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                input = input.replace("\n", "");

                println!("input to write {input}");
                pwm.write_all(input.as_bytes()).expect("Cannot write to file");
                pwm.flush();

                input.clear();
            }
            Err(error) => println!("error: {error}"),
        }
    }
}
