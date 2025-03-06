use std::process::Command;
use std::thread::sleep;

fn main() {
    //The plan is to make a program here that we can run when starting the computer, but we have not used it much yet
    Command::new("cargo")
        .args(["run", "--bin", "master_main"])
        .spawn()
        .expect("Failed to start master_main");

    sleep(std::time::Duration::from_secs(1));

    Command::new("cargo")
        .args(["run", "--bin", "slave_main"])
        .spawn()
        .expect("Failed to start slave_main");

    // sleep(std::time::Duration::from_secs(10));
}
