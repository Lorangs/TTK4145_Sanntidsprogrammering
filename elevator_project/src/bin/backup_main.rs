use elevator_project::{backup::Backup, config::Config, master::Master};
use std::path::Path;

fn main() {
    let config = Config::config(Path::new("config.json")).unwrap();
    let backup_ip = config.elevator_ip_list[1].to_string() + ":" + &config.backup_port.to_string();

    loop {
        let mut backup = Backup::init(&config);
        let masterqueues = backup.backup_loop();
        let mut master = Master::init(&config, masterqueues).unwrap();
        master.master_loop();
    }
}
