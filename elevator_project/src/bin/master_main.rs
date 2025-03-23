use elevator_project::config::Config;
use elevator_project::master::{Master, OrderRequests};
use elevator_project::backup::Backup;
use std::path::Path;

fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();

    //starting a master first, and if it fails be ready as a backup
    let mut master = Master::init(&config, OrderRequests::init()).unwrap();
    master.master_loop();

    loop {
        let mut backup = Backup::init(&config);
        let result = backup.backup_loop();
        
        match result {
            Ok(order_requests) => {
                let mut master = Master::init(&config, order_requests).unwrap();
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
