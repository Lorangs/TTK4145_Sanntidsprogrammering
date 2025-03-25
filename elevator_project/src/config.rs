use serde::{Deserialize, Serialize};
use std::fs::File;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufReader, Error};
use std::net::Ipv4Addr;
use std::path::Path;
use std::result::Result;

// constants variables for the elevator system
// Struggled to put these in the config file due to compile time errors
pub const NUMBER_OF_FLOORS      : usize = 4;
pub const NUMBER_OF_ELEVATORS   : usize = 3;

// Custom serde module for Vec<Ipv4Addr> serialization/deserialization
mod ipv4_address_vec {
    use serde::{Deserialize, Deserializer, Serializer, Serialize};
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    use super::NUMBER_OF_ELEVATORS;

    pub fn serialize<S>(addresses: &[Ipv4Addr;NUMBER_OF_ELEVATORS], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Ipv4Addr to strings for serialization
        let str_addresses: Vec<String> = addresses
            .iter()
            .map(|addr| addr.to_string())
            .collect();
        str_addresses.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[Ipv4Addr; NUMBER_OF_ELEVATORS], D::Error>
    where
        D: Deserializer<'de>,
    {
        let str_addresses: Vec<String> = Vec::deserialize(deserializer)?;
        
        // Convert strings to Ipv4Addr
        let vec: Result<Vec<Ipv4Addr>, _> = str_addresses
            .iter()
            .map(|s| Ipv4Addr::from_str(s))
            .collect();

        // Ensure the vector has the correct length
        if vec.clone().unwrap().len() != NUMBER_OF_ELEVATORS {
            return Err(serde::de::Error::custom(format!(
                "Expected {} IP addresses, but got {}",
                NUMBER_OF_ELEVATORS,
                vec.unwrap().len()
            )));
        }

        // Convert Vec to array
        let mut array = [Ipv4Addr::new(0, 0, 0, 0); NUMBER_OF_ELEVATORS];
        for (i, addr) in vec.unwrap().into_iter().enumerate() {
            array[i] = addr;
        }
        Ok(array)
    }
}


// Maps the config file to a struct
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    #[serde(with = "ipv4_address_vec")]
    pub elevator_ip_list: [Ipv4Addr; NUMBER_OF_ELEVATORS],
    pub master_port: u16,
    pub backup_port: u16,
    pub slave_port: u16,
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
            Slave port:\t\t\t{}\n\
            Door open duration [s]:\t\t{}\n\
            Input poll rate [ms]:\t\t{}\n\
            TCP timeout [ms]:\t\t{}",
            self.elevator_ip_list,
            self.master_port,
            self.backup_port,
            self.slave_port,
            self.door_open_duration_s,
            self.input_poll_rate_ms,
            self.tcp_timeout_ms
        )
    }
}

impl Config {
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
