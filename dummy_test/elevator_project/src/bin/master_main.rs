#![allow(warnings)]

use elevator_project::config::Config;
use elevator_project::master::Master;
use std::path::Path;



fn main() {
    let config = Config::config(Path::new("config.json")).unwrap();
    let master_ip   = config.elevator_ip_list[0].to_string() + ":" + &config.master_port.to_string();

    let mut master = Master::init(&config, &master_ip, master::M).unwrap();
    print!("Master initialized\n");
    
    master.master_loop();
    
      
    
}