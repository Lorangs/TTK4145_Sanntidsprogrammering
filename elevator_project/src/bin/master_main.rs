use elevator_project::config::Config;
use elevator_project::master::{Master, OrderRequests};
use std::path::Path;
use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let mut order_requests_json=OrderRequests::init();
    if args.len() < 2 {
        println!("No order requests provided");
    }
    else{
        //let reader = BufReader::new(Cursor::new(args[1].clone()));
        order_requests_json = serde_json::from_str(&args[1]).unwrap();
        println!("[MASTER]\tOrder requests: {:?}", order_requests_json);
    }

    //starting a master first, and if it fails be ready as a backup
    let mut master = Master::init(&config, order_requests_json).unwrap();
    println!("[MASTER]\tMaster initialized");
    master.master_loop();

    Command::new("cargo")
        .args(["run", "--bin", "backup_main"])
        .spawn()
        .expect("Failed to start backup_main");
}
