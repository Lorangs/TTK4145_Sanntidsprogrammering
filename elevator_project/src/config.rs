use serde::{Deserialize, Serialize};
use std::fs::File;

use std::path::Path;  
use std::io::BufReader; 
use std::io::Error;
use std::result::Result; 
use std::fmt;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub elevator_ip_list        : Vec<String>,
    pub master_port             : u16,
    pub backup_port             : u16,
    pub slave_port              : u16,
    pub number_of_floors        : u8,
    pub number_of_elevators     : u8,
    pub door_open_duration_s    : f32,
    pub input_poll_rate_ms      : u64,
    pub tcp_timeout_ms          : u64,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"Elevator IP list:\t\t{:?}\nMaster port:\t\t\t{}\nBackup port:\t\t\t{}\nSlave port:\t\t\t{}\nNumber of floors:\t\t{}\nNumber of elevators:\t\t{}\nDoor open duration [s]:\t\t{}\nInput poll rate [ms]:\t\t{}",
            self.elevator_ip_list, 
            self.master_port, 
            self.backup_port, 
            self.slave_port, 
            self.number_of_floors, 
            self.number_of_elevators, 
            self.door_open_duration_s, 
            self.input_poll_rate_ms)
    }
}

impl Config {
    pub fn config(path: &Path) -> Result<Config, Error> {        
        println!("[CONFIG]\tReading config file");
        let file = match File::open(path){
            Ok(file) => file,
            Err(e) => {
                panic!("[CONFIG]\tFailed to open file: {}", e);
            },
        };  
        let reader = BufReader::new(file);
        let config: Config = serde_json::from_reader(reader)?;
        
        println!("[CONFIG]\tConfig loaded successfully:\n\n{}", config);
        return Ok(config);
    }
}    



