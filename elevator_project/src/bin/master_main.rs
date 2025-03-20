use elevator_project::config::Config;
use elevator_project::master::{Master, MasterQueues};
use elevator_project::backup::Backup;
use std::path::Path;

fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let mut masterqueues: MasterQueues = MasterQueues::init();

    //starting a master first, and if it fails be ready as a backup
    let mut master = Master::init(&config, masterqueues).unwrap();
    master.master_loop();

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
