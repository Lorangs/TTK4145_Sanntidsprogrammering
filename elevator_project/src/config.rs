use serde::{Deserialize, Serialize};
use std::fs::File;

use std::path::Path;  
use std::io::BufReader; 
use std::io::Error;


#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub elevator_ip_list: Vec<String>,
    pub master_port: u16,
    pub backup_port: u16,
    pub slave_port: u16,
    pub number_of_floors: u8,
    pub door_open_duration_s: f32,
    pub input_poll_rate_ms: u64,
}

impl Config{
    pub fn ip_to_string(&self, ) -> String {
        format!("{}:{}", ip, port)
    }
    
    pub fn config(path: &Path) -> std::result::Result<Config, Error> {        
        println!("[CONFIG]\tReading config file");
        let file = match File::open(path){
            Ok(file) => file,
            Err(e) => {
                panic!("[CONFIG]\tFailed to open file: {}", e);
            },
        };  
        let reader = BufReader::new(file);
        let config: Config = serde_json::from_reader(reader)?;
        
        println!("[CONFIG]\tConfig loaded successfully");
        return Ok(config);
    }

    pub fn print(&self) {
        println!("{:?}", self.elevator_ip_list);
        println!("{}", self.master_port);
        println!("{}", self.backup_port);
        println!("{}", self.slave_port);
        println!("{}", self.number_of_floors);
        println!("{}", self.door_open_duration_s);
        println!("{}", self.input_poll_rate_ms);
    }
}    



