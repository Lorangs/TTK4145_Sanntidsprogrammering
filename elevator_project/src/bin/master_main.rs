use elevator_project::config::Config;
use elevator_project::master::{self, Master};
use std::path::Path;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::read_config(Path::new("config.json")).unwrap();
    let master_ip = config.elevator_ip_list[args[1].parse::<usize>().unwrap()].to_string() + ":" + &config.master_port.to_string();

    loop {
        let mut master = Master::init(&config, masterqueues).unwrap();
        master.master_loop();
        let mut backup = Backup::init(&config);
        let masterqueues = backup.backup_loop();
    }
}
