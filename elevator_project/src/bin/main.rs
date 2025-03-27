use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    //Start as master
    if args[1] == 0.to_string() {
        Command::new("cargo")
            .args(["run", "--bin", "master_main"])
            .spawn()
            .expect("Failed to start master_main");
    } else
    //Start backup
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
