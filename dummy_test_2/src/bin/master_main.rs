#![allow(warnings)]

use dummy_test_2::master::Master;
use dummy_test_2::config::Config;
use std::path::Path;



fn main() {
    let config = Config::config(Path::new("config.json")).unwrap();
    let master_ip   = config.elevator_ip_list[0].to_string() + ":" + &config.master_port.to_string();

    let mut master = Master::init(&config, &master_ip);
    print!("[MASTER]\tMaster initialized\n");
    
    master.master_loop();
    
      
    
}