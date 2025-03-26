use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use debug_print::debug_println as dprintln;



fn main() {
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let slave_ip = "localhost".to_string() + ":" + &config.slave_port.to_string();  
    //ska vi omdøpe slaveport til elevator_port??? vist du e enig kan du endre

    let mut slave = Slave::init(slave_ip.clone(), &config);
    dprintln!("Slave initialized\n");

    slave.slave_loop();
    dprintln!("[SLAVE]\t\tslave failed, restarting slave\n");
    
    //drop(slave);
    //sleep(Duration::from_secs(1)); Testa uten dei og det funka, men har lyst å teste en gang til før eg sletta

    Command::new("cargo")
        .args(["run", "--bin", "slave_main"])
        .spawn()
        .expect("Failed to start slave_main");
    
}
