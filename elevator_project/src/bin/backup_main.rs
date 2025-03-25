use elevator_project::{backup::Backup, config::Config, master::Master};
use std::path::Path;

// Main func for backup. Initializes backup. If backup crashes, it will restart and connect to a new master. 
// We need to seperate between the backup crashing: restart as backup, and master crashing: start a master.
fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();

    loop {
        let mut backup = Backup::init(&config);
        let masterqueues_result = backup.backup_loop();
        
        match masterqueues_result {
            Ok(masterqueues) => {
                let mut master = Master::init(&config, masterqueues).unwrap();
                println!("[MASTER]\tMaster initialized");
                master.master_loop();
            }
            Err(e) => {
                println!("[BACKUP]\tBackup loop failed: {:?}", e);
                println!("[BACKUP]\tRestarting backup");
            }
        }
    }
}
