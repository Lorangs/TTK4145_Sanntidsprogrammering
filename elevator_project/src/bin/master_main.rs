use elevator_project::config::Config;
use elevator_project::master::{self, Master};
use std::path::Path;

fn main() {
    let config = Config::config(Path::new("config.json")).unwrap();
    let master_ip = config.elevator_ip_list[0].to_string() + ":" + &config.master_port.to_string();

    let mut master = Master::init(&config, master::MasterQueues::init()).unwrap();
    print!("[MASTER]\tMaster initialized\n");

    master.master_loop();
}
