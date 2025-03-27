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

    if args.len() < 2 {
        dprintln!("[MASTER]\tNo order requests provided. Starting with empty order requests");
    }
    else{
        //let reader = BufReader::new(Cursor::new(args[1].clone()));
        order_requests = serde_json::from_str(&args[1]).unwrap();
        dprintln!("[MASTER]\tOrder requests: {:?}", order_requests);
    }

    //starting a master first, and if it fails start up again as backup
    let mut master = Master::init(&config, order_requests).unwrap();
    dprintln!("[MASTER]\tMaster initialized");
    master.master_loop();

    Command::new("cargo")
        .args(["run", "--bin", "backup_main"])
        .spawn()
        .expect("Failed to start backup_main");
}
