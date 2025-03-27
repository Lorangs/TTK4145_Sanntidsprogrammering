use debug_print::debug_println as dprintln;
use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::path::Path;
use std::process::Command;

/// Main function for the slave unit.
/// If the loop exits (e.g., due to an error), a new slave process is spawned to restart it.
fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();

    dprintln!("trying to start a slave\n");

    let mut slave = Slave::init(&config);
    dprintln!("Slave initialized\n");

    slave.slave_loop();
    dprintln!("[SLAVE]\t\tslave failed, restarting slave\n");

    Command::new("cargo")
        .args(["run", "--bin", "slave_main"])
        .args(["run", "--bin", "slave_main"])
        .spawn()
        .expect("Failed to start slave_main");
}
