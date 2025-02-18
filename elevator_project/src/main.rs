mod config;
mod master;
mod slave;
mod inputs;
mod tcp;

use crate::master::Master;
use crate::config::Config;
use std::path::Path;
use std::thread::spawn;

fn main() {
    let config = config::Config::config(Path::new("config.json")).unwrap();

    let master_ip   = config.elevator_ip_list[0].to_string() + ":" + &config.master_port.to_string();
    let slave_ip    = config.elevator_ip_list[0].to_string() + ":" + &config.slave_port.to_string();


    let mut master = master::Master::init(&config, &master_ip).unwrap();
    print!("Master initialized\n");

    master.master_loop();
      
    
}