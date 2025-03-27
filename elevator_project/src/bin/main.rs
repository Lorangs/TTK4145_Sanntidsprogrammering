use std::env;
use std::process::Command;

/// Main program. Spawn two different program threads. One master / backup, depending on input argument, and one slave unit. 
/// Start as master + slave if 0 is passed as first argument.
/// Start as backup + slave otherwise.
fn main() {
    let args: Vec<String> = env::args().collect();


    if args[1] == 0.to_string() 
    {
        Command::new("cargo")
            .args(["run", "--bin", "master_main"])
            .spawn()
            .expect("Failed to start master_main");
    } 
    else
    {
        Command::new("cargo")
            .args(["run", "--bin", "backup_main"])
            .spawn()
            .expect("Failed to start backup_main");
    }

    Command::new("cargo")
        .args(["run", "--bin", "slave_main"])
        .spawn()
        .expect("Failed to start slave_main");
}
