use std::process::Command;
use std::thread::sleep;
use std::env;
use std::path::Path;



fn main() {
    let args: Vec<String> = env::args().collect();

    // start as master
    if args[1] == 0.to_string()
    {
        Command::new("cargo")
        .args(["run", "--bin", "master_main", args[1].as_str()])
        .spawn()
        .expect("Failed to start master_main");
    }
    else // start backup
    {
        Command::new("cargo")
        .args(["run", "--bin", "backup_main", args[1].as_str()])
        .spawn()
        .expect("Failed to start backup_main");
    }

    sleep(std::time::Duration::from_secs(1));

    Command::new("cargo")
        .args(["run", "--bin", "slave_main", args[1].as_str()])
        .spawn()
        .expect("Failed to start slave_main");
}
