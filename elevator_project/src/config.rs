use serde::{Deserialize, Serialize};
use std::fs::File;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::BufReader;
use std::io::Error;
use std::path::Path;
use std::result::Result;

// Maps the config file to a struct
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub elevator_ip_list: Vec<String>,
    pub master_port: u16,
    pub backup_port: u16,
    pub number_of_floors: u8,
    pub number_of_elevators: u8,
    pub door_open_duration_s: f32,
    pub input_poll_rate_ms: u64,
    pub tcp_timeout_ms: u64,
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Elevator IP list:\t\t{:?}\n\
            Master port:\t\t\t{}\n\
            Backup port:\t\t\t{}\n\
            Number of floors:\t\t{}\n\
            Number of elevators:\t\t{}\n\
            Door open duration [s]:\t\t{}\n\
            Input poll rate [ms]:\t\t{}",
            self.elevator_ip_list,
            self.master_port,
            self.backup_port,
            self.number_of_floors,
            self.number_of_elevators,
            self.door_open_duration_s,
            self.input_poll_rate_ms
        )
    }
}

impl Config {

    // Reads the config file and returns a Config struct
    pub fn read_config(path: &Path) -> Result<Config, Error> {
        println!("[CONFIG]\tReading config file");
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                panic!("[CONFIG]\tFailed to open file: {}", e);
            }
        };
        let reader = BufReader::new(file);
        let config: Config = serde_json::from_reader(reader)?;

        println!("[CONFIG]\tConfig loaded successfully:\n{}", config);
        return Ok(config);
    }
}
