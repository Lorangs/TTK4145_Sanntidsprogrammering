use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::env;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use debug_print::debug_println as dprintln;



fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let slave_ip = config.elevator_ip_list[args[1].parse::<usize>().unwrap()].to_string() + ":" + &config.slave_port.to_string();
    
    
    dprintln!("trying to start a slave\n");    

    let mut slave = Slave::init(slave_ip.clone(), &config);
    dprintln!("Slave initialized\n");

    slave.slave_loop();
    dprintln!("[SLAVE]\t\tslave failed, restarting slave\n");
    drop(slave);

    sleep(Duration::from_secs(1));

    Command::new("cargo")
        .args(["run", "--bin", "slave_main", args[1].as_str()])
        .spawn()
        .expect("Failed to start slave_main");
    
}

//TODO try to restart slave if it crashes
