use debug_print::debug_println as dprintln;
use elevator_project::config::Config;
use elevator_project::master::{Master, OrderRequests};
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::read_config(Path::new("config.json")).unwrap();

    let mut order_requests_json = OrderRequests::init();

    if args.len() > 2 {
        //master was started by a slave
        order_requests_json = serde_json::from_str(&args[1]).unwrap();
        dprintln!("[MASTER]\tOrder requests: {:?}", order_requests_json);
    }

    //starting a master first, and if it fails be ready as a backup
    let mut master = Master::init(&config, order_requests_json).unwrap();
    dprintln!("[MASTER]\tMaster initialized");
    master.master_loop();

    Command::new("cargo")
        .args(["run", "--bin", "backup_main"])
        .spawn()
        .expect("Failed to start backup_main");
}
