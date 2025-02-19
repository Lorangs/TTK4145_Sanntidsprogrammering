use slave::Slave;
use config::Config;
use std::path::Path;
use std::thread::spawn;

fn main() {
    let config = config::Config::config(Path::new("config.json")).unwrap();

    let master_ip   = config.elevator_ip_list[0].to_string() + ":" + &config.master_port.to_string();
    let slave_ip    = config.elevator_ip_list[0].to_string() + ":" + &config.slave_port.to_string();


    let mut slave = slave::Slave::init(slave_ip, &config);
    print!("Slave initialized\n");

    slave.slave_loop();
}