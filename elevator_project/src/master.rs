use queues::*;


use crate::elev::config::Config;

// Struct for slave data
pub struct SlaveData {
    pub ip: String,                             // IP address of slave
    pub buffer: Buffer<u8>,                     // Buffer for orders, max capacity 255  
}

impl SlaveData {
    // Constructor
    pub fn new(ip: String) -> Self {
        Self {
            ip: ip,
            buffer: Buffer::new(255),       
        }
    }
    pub fn add_order(&mut self, order: u8) {
        self.buffer.add(order);
    }
    pub fn remove_order(&mut self) -> u8 {
        self.buffer.remove().unwrap()
    }
    pub fn is_empty(&self) -> bool {
        if self.buffer.size() == 0 { return true }
        else { return false }
    }

    pub fn print(&self) {
        println!("Buffer for slave {}: ", self.ip);
        println!("{:?}", self.buffer);
    }
}


struct Master {
    config: Config,     
    backup: String,             // IP address of backup
    slaves: Vec<SlaveData>,     // Vector of slaves

    
}