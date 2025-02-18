mod config;
mod test;
mod slave;
mod master;
mod tcp;
mod inputs;

use std::thread::spawn;
use std::path::Path;
use driver_rust::elevio;
use driver_rust::elevio::elev as e;



fn main() {
    let config = config::Config::config(Path::new("config.json")).unwrap();

    let master_ip   = config.elevator_ip_list[0].to_string() + ":" + &config.master_port.to_string();
    let slave_ip    = config.elevator_ip_list[0].to_string() + ":" + &config.slave_port.to_string();




    let mut slave = slave::Slave::init(slave_ip, &config);
    print!("Slave initialized\n");



    spawn(move || {
        slave.slave_loop();
    });
      
    

}