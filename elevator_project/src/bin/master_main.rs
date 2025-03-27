use debug_print::debug_println as dprintln;
use elevator_project::config::Config;
use elevator_project::master::{Master, OrderRequests};
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::read_config(Path::new("config.json")).unwrap();

    let mut order_requests = OrderRequests::init();

    println!("Argument: {}",args.len());

    if args.len() != 1 {
        //master was started by a backup
        match (order_requests = serde_json::from_str(&args[1])){
            Ok()=>{}
            Err()=>{
                Command::new("cargo")
                    .args(["run", "--bin", "master_main", args[1]])
                    .spawn()
                    .expect("Failed to start master_main");
                return;
            }
        }
        dprintln!("[MASTER]\tOrder requests: {:?}", order_requests);
    }
    else{
        dprintln!("I am a new master");

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
