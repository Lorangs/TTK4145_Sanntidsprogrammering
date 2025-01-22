use serde::{Deserialize, Serialize};
use std::fs::File;

use std::path::Path;  
use std::io::BufReader; 
use std::io::{self, Read, Error};


#[derive(Deserialize, Serialize)]
pub struct Config {
    pub elevator_ip_list: Vec<String>,
    pub number_of_floors: u8,
    pub door_open_duration_s: f32,
    pub input_poll_rate_ms: u64
}

pub fn load_config(path: &Path) -> std::result::Result<Config, Error> {
    println!("Reading config file");
    let file = match File::open(path){
        Ok(file) => file,
        Err(e) => panic!("Failed to open file: {}", e)
    };
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader)?;
    
    println!("{:?}", config.elevator_ip_list);
    println!("Config loaded successfully");
    return Ok(config);
}