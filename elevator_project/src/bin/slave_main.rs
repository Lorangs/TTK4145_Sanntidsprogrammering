use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("IP:\t{}", &args[1].to_string());
    print!("trying to start a slave\n");

    let config = Config::read_config(Path::new("config.json")).unwrap();

    let slave_ip = config.elevator_ip_list[0].to_string() + ":" + &args[1].to_string();

    let mut slave = Slave::init(slave_ip, &config);
    print!("Slave initialized\n");

    slave.slave_loop();
}

//TODO try to restart slave if it crashes
