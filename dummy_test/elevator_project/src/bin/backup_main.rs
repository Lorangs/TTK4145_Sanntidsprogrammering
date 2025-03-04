#![allow(warnings)]

use elevator_project::config::Config;
use elevator_project::backup::Backup;
use std::path::Path;



fn main() {
    let config = Config::config(Path::new("config.json")).unwrap();
    let backup_ip   = config.elevator_ip_list[1].to_string() + ":" + &config.backup_port.to_string();

    let mut backup = Backup::init(&config);

    let backup.backup_loop(); 

    

        
}