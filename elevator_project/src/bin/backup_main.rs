use elevator_project::{backup::Backup, config::Config, master::Master};
use std::path::Path;
use std::env;


// Main func for backup. Initializes backup. If backup crashes, it will restart and connect to a new master. 
// We need to seperate between the backup crashing: restart as backup, and master crashing: start a master.
fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::read_config(Path::new("config.json")).unwrap();
    let backup_ip = config.elevator_ip_list[args[1].parse::<usize>().unwrap()].to_string() + ":" + &config.backup_port.to_string();

    loop {
        let mut backup = Backup::init(&config);
        let masterqueues = backup.backup_loop();
        let mut master = Master::init(&config, masterqueues).unwrap();
        print!("[MASTER]\tMaster initialized\n");
        master.master_loop();
    }
}
