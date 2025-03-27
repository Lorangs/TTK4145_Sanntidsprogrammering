use elevator_project::config::Config;
use elevator_project::master::{Master, OrderRequests};
use std::path::Path;
use std::env;
use std::process::Command;
use debug_print::debug_println as dprintln;


fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let mut order_requests=OrderRequests::init();

    // Take order requests from command line arguments as input to the master if provided. 
    // If none provided, start with an empty order requests.
    if args.len() < 2 {
        dprintln!("[MASTER]\tNo order requests provided. Starting with empty order requests");
    }
    else{
        order_requests = OrderRequests::from_json_string(&args[1]);
        dprintln!("[MASTER]\tOrder requests: {:?}", order_requests);
    }

    // Initialize master and start master loop.
    let mut master = Master::init(&config, order_requests).unwrap();
    dprintln!("[MASTER]\tMaster initialized");
    master.master_loop();

    // If master loop returns, master has failed. Start as backup instead.
    dprintln!("[MASTER]\tMaster failed, restarting as backup");
    drop(master);
    Command::new("cargo")
        .args(["run", "--bin", "backup_main"])
        .spawn()
        .expect("Failed to start backup_main");
}
