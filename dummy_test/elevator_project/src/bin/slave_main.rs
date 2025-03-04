#![allow(warnings)]

use elevator_project::slave::Slave;
use elevator_project::config::Config;
use std::path::Path;
use std::thread::spawn;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("IP:\t{}",&args[1].to_string());
    print!("trying to start a slave\n");
    
    let config = Config::config(Path::new("config.json")).unwrap();

    let slave_ip = config.elevator_ip_list[0].to_string() + ":" + &args[1].to_string();

    let mut slave = Slave::init(slave_ip, &config);
    print!("Slave initialized\n");

    slave.slave_loop();
}

