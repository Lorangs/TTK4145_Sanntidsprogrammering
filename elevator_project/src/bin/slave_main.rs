use debug_print::debug_println as dprintln;
use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::path::Path;
use std::process::Command;

fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let elevator_ip = "localhost".to_string() + ":" + &config.elevator_port.to_string();

    let mut slave = Slave::init(elevator_ip, &config);
    dprintln!("Slave initialized\n");

    slave.slave_loop();
    dprintln!("[SLAVE]\t\tslave failed, restarting slave\n");

    Command::new("cargo")
        .args(["run", "--bin", "slave_main"])
        .spawn()
        .expect("Failed to start slave_main");
}
