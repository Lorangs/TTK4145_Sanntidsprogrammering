use std::String
use std::net::{TcpListener, TcpStream};
use crate::config::Config;


struct backup {
    master_socket           : TcpStream,
}

impl backup {
    pub fn init(config: Config) -> backup {
        let master_listener
    }
}