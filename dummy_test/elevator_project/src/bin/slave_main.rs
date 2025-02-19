#![allow(warnings)]

use elevator_project::slave::Slave;
use elevator_project::config::Config;
use std::path::Path;
use std::thread::spawn;

fn main() {
    print!("trying to start a slave\n");
    
    let config = Config::config(Path::new("config.json")).unwrap();

    let slave_ip    = config.elevator_ip_list[0].to_string() + ":" + "15657";

    let mut slave = Slave::init(slave_ip, &config);
    print!("Slave initialized\n");

    slave.slave_loop();
}