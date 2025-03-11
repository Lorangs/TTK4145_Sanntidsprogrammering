#![allow(warnings)]

use dummy_test_2::config::Config;
use dummy_test_2::backup::Backup;
use std::path::Path;



fn main() {
    let config = Config::config(Path::new("config.json")).unwrap();
    let backup_ip   = config.elevator_ip_list[1].to_string() + ":" + &config.backup_port.to_string();

    let mut backup = Backup::init(&config);
    println!("[BACKUP]\tBackup initialized\n");
    backup.backup_loop(); 
        
}