use serde::{Deserialize, Serialize};
use std::fs::File;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::BufReader;
use std::io::Error;
use std::net::Ipv4Addr;
use std::path::Path;
use std::result::Result;
use std::str::FromStr;




// Custom serde module for Vec<Ipv4Addr> serialization/deserialization
mod ipv4_address_vec {
    use serde::{Deserialize, Deserializer, Serializer, Serialize};
    use std::net::Ipv4Addr;
    use std::str::FromStr;

    pub fn serialize<S>(addresses: &Vec<Ipv4Addr>, serializer: S) -> Result<S::Ok, S::Error>
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

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Ipv4Addr>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str_addresses: Vec<String> = Vec::deserialize(deserializer)?;
        
        // Convert strings to Ipv4Addr
        let result: Result<Vec<Ipv4Addr>, _> = str_addresses
            .iter()
            .map(|s| Ipv4Addr::from_str(s).map_err(serde::de::Error::custom))
            .collect();
            
        result
    }
}


// Maps the config file to a struct
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    #[serde(with = "ipv4_address_vec")]
    pub elevator_ip_list: Vec<Ipv4Addr>,
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
            Input poll rate [ms]:\t\t{}\n\
            TCP timeout [ms]:\t\t{}",
            self.elevator_ip_list,
            self.master_port,
            self.backup_port,
            self.number_of_floors,
            self.number_of_elevators,
            self.door_open_duration_s,
            self.input_poll_rate_ms,
            self.tcp_timeout_ms
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


  
