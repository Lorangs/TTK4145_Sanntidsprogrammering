use std::process::Command;
use std::thread::sleep;

fn main() {
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
