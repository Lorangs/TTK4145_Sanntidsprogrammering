use elevator_project::{backup::Backup, config::Config};
use std::path::Path;
use std::process::Command;
use debug_print::debug_println as dprintln;


// Main func for backup. Initializes backup. If backup crashes, it will restart and connect to a new master. 
// We need to seperate between the backup crashing: restart as backup, and master crashing: start a master.
fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();

    let mut backup = Backup::init(&config);
    let masterqueues_result = backup.backup_loop();
    
    match masterqueues_result {
        Ok(masterqueues) => {
            Command::new("cargo")
                .args(["run", "--bin", "master_main", masterqueues.to_custom_json().as_str()])
                .spawn()
                .expect("Failed to start master_main");
        }
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
