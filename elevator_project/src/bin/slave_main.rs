use elevator_project::config::Config;
use elevator_project::slave::Slave;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    print!("trying to start a slave\n");

    let config = Config::read_config(Path::new("config.json")).unwrap();

    loop{
        let slave_ip = config.elevator_ip_list[args[1].parse::<usize>().unwrap()].to_string() + ":" + &config.slave_port.to_string();

        let mut slave = Slave::init(slave_ip, &config);
        print!("Slave initialized\n");

        slave.slave_loop();
        print!("slave faled, restarting slave\n");
    }
}

//TODO try to restart slave if it crashes
