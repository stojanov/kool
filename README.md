
# kool
## Fan control background service for linux

![Build status](https://github.com/stojanov/kool/actions/workflows/rust.yml/badge.svg)

Kool is a very simple and straightforward software for fan controlling on linux.
Written for me to exmeriment/learn rust.
Registering with your init system has to be manually, currenty i don't have install scrips provided, subject to change
## Features

- Toml config file
- Custom source temperature (suported a file and program)
- Lightweight
- Multithreaded runner and guaranteed correct polling time

#### Building for source
```sh
# clone the repo and just run inside the directory
cargo build
```
## Example config file
```toml
[[control]]
name = "gpu_control"
interval = 1000 # in milliseconds
src_path = "/usr/bin/gpu-usage" # source of value temperature
src_args = [ "temp" ] # only used if running a program/script
src_type = "program" # file(as in file to read from), script, program 
dest_path = "/sys/class/hwmon/hwmon0/pwm4" # destination to write pwm, usually [0, 255]
dest_min = 0
dest_max = 255  
default_dest_percent = 40 # in case of crash, this will be the default percentage, if the src crashes, you will be notified and the control will stop polling (not implemented)
curve = "linear" (not implemented)
points = [
    [30, 30], # [ source temp, output to the pwm in percent]
    [60, 60],
    [90, 90],
]
```

## License

GPL

**Free Software, Hell Yeah!**
