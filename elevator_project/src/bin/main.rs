use std::process::Command;

fn main() {
    Command::new("cargo")
        .args(["run", "--bin", "master_main"])
        .spawn()
        .expect("Failed to start master_main");

    Command::new("cargo")
        .args(["run", "--bin", "slave_main"])
        .spawn()
        .expect("Failed to start slave_main");
}