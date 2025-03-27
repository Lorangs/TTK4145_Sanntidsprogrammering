use debug_print::debug_println as dprintln;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs::File;
use std::io::{BufReader, Error};
use std::net::Ipv4Addr;
use std::path::Path;
use std::result::Result;

// Constant variables for the elevator system.
// Could not put these in the config file due to compile time errors.
pub const NUMBER_OF_FLOORS: usize = 4;
pub const NUMBER_OF_ELEVATORS: usize = 3;
pub const BUFFER_SIZE: usize = 128;

/// Custom serde module for Vec<Ipv4Addr> serialization/deserialization
mod ipv4_address_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    use super::NUMBER_OF_ELEVATORS;

    pub fn serialize<S>(
        addresses: &[Ipv4Addr; NUMBER_OF_ELEVATORS],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Ipv4Addr to strings for serialization
        let str_addresses: Vec<String> = addresses.iter().map(|addr| addr.to_string()).collect();
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

/// Maps the config file to a struct
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    #[serde(with = "ipv4_address_vec")]
    pub elevator_ip_list: [Ipv4Addr; NUMBER_OF_ELEVATORS],
    pub master_port: u16,
    pub backup_port: u16,
    pub elevator_port: u16,
    pub door_open_duration_s: u64,
    pub input_poll_rate_ms: u64,
    pub tcp_timeout_ms: u64,
    pub est_moving_time_s: u64,
}
impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Config:\n\
            Elevator IP list:\t\t{:?}\n\t\
            Master port:\t\t\t{}\n\t\
            Backup port:\t\t\t{}\n\t\
            Elevator port:\t\t\t{}\n\t\
            Door open duration [s]:\t\t{}\n\t\
            Input poll rate [ms]:\t\t{}\n\t\
            TCP timeout [ms]:\t\t{}\n\t\
            Estimated moving time [s]:\t{}",
            self.elevator_ip_list,
            self.master_port,
            self.backup_port,
            self.elevator_port,
            self.door_open_duration_s,
            self.input_poll_rate_ms,
            self.tcp_timeout_ms,
            self.est_moving_time_s
        )
    }
}
impl Config {
    /// Reads the config file and returns a Config struct
    pub fn read_config(path: &Path) -> Result<Config, Error> {
        dprintln!("[CONFIG]\t\tReading config file");
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                panic!("[CONFIG]\t\tFailed to open file: {}", e);
            }
        };
        let reader = BufReader::new(file);
        let config: Config = serde_json::from_reader(reader)?;

        dprintln!("[CONFIG]\t\tConfig loaded successfully:\n{}", config);
        Ok(config)
    }
}
