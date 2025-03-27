use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use debug_print::debug_println as dprintln;



fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();
    
    dprintln!("trying to start a slave\n");    

    let mut slave = Slave::init(&config);
    dprintln!("Slave initialized\n");

    slave.slave_loop();
    dprintln!("[SLAVE]\t\tslave failed, restarting slave\n");
    drop(slave);

    sleep(Duration::from_secs(1));

    Command::new("cargo")
        .args(["run", "--bin", "slave_main"])
        .spawn()
        .expect("Failed to start slave_main");
    
}
