use elevator_project::config::Config;
use elevator_project::master::{Master, MasterQueues};
use elevator_project::backup::Backup;
use std::path::Path;

fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let mut masterqueues: MasterQueues = MasterQueues::init();

    loop {
        let mut master = Master::init(&config, masterqueues).unwrap();
        master.master_loop();
        let mut backup = Backup::init(&config);
        masterqueues = backup.backup_loop();
    }
}
