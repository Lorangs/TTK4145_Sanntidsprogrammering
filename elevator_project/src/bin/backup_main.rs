use elevator_project::{backup::Backup, config::Config};
use std::path::Path;
use std::process::Command;
use debug_print::debug_println as dprintln;

fn main() {
    let config = Config::read_config(Path::new("config.json")).expect("Failed to read config.json");

    let mut backup = Backup::init(&config).expect("Failed to initialize backup");
    let masterqueues_result = backup.backup_loop();
    
    match masterqueues_result {
        //Backup loop ended because it lost conection to master -> start a new master
        Ok(masterqueues) => {
            Command::new("cargo")
                .args(["run", "--bin", "master_main", masterqueues.to_custom_json().as_str()])
                .spawn()
                .expect("Failed to start master_main");
        }
        //Backup loop ended because of an error -> start a new backup
        Err(e) => {
            dprintln!("[BACKUP]\tBackup loop failed: {:?}", e);
            dprintln!("[BACKUP]\tRestarting backup");
            Command::new("cargo")
                .args(["run", "--bin", "backup_main"])
                .spawn()
                .expect("Failed to start backup_main");
        }
    }
}