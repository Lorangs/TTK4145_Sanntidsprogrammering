#![allow(warnings)]

use dummy_test_2::slave::Slave;
use dummy_test_2::config::Config;
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

//Standar heis: "127.0.0.1.15657"
//Heis 2: "127.0.0.1.15658"