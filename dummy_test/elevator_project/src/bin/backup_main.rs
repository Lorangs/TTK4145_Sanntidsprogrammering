#![allow(warnings)]

use elevator_project::{config::Config, backup::Backup, master::Master};
use std::path::Path;
use std::env;


fn main() {

    let config = Config::config(Path::new("config.json")).unwrap();
    let backup_ip   = config.elevator_ip_list[1].to_string() + ":" + &config.backup_port.to_string();

    
    loop {
        let mut backup = Backup::init(&config);
        let mut masterqueues = backup.backup_loop(); 
        let mut master = Master::init(&config, &backup_ip, masterqueues).unwrap();
        master.master_loop();
    }
        
}