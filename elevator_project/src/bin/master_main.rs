use debug_print::debug_println as dprintln;
use elevator_project::config::Config;
use elevator_project::io_datastructures::OrderRequests;
use elevator_project::master::Master;
use std::env;
use std::path::Path;
use std::process::Command;

/// Main function for the Master unit.
/// If starting from scratch takes no argument.
/// Else, if started by a backup, takes a json string of type OrderRequests as input argument.
fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::read_config(Path::new("config.json")).unwrap();

    let mut order_requests = OrderRequests::init();

    dprintln!("Argument:\t{}", args.len());

    if args.len() != 1 {
        match serde_json::from_str(&args[2]) {
            Ok(or) => order_requests = or,
            Err(_e) => {
                Command::new("cargo")
                    .args(["run", "--release", "--bin", "master_main", &args[2]])
                    .spawn()
                    .expect("Failed to start master_main");
                return;
            }
        }
        dprintln!("[MASTER]\tOrder requests:\t{:?}", order_requests);
    } else {
        dprintln!("I am a new master");
    }

    let mut master = Master::init(&config, order_requests).unwrap();
    dprintln!("[MASTER]\tMaster initialized");
    master.master_loop();

    // If master loop returns, master has failed. Start as backup instead.
    dprintln!("[MASTER]\tMaster failed, restarting as backup");
    drop(master);
    Command::new("cargo")
        .args(["run", "--release", "--bin", "backup_main"])
        .spawn()
        .expect("Failed to start backup_main");
}
