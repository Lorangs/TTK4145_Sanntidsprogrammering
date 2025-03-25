use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::read_config(Path::new("config.json")).unwrap();
    let slave_ip = config.elevator_ip_list[args[1].parse::<usize>().unwrap()].to_string() + ":" + &config.slave_port.to_string();
    
    loop{
        print!("trying to start a slave\n");    

        let mut slave = Slave::init(slave_ip.clone(), &config);
        print!("Slave initialized\n");

        slave.slave_loop();
        print!("[SLAVE]\t\tslave failed, restarting slave\n");
        drop(slave);
    }
}

//TODO try to restart slave if it crashes
