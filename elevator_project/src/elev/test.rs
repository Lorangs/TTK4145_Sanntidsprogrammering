// Denne siden brukes til test av diverse funksjoner i programmet

use crate::elev::*;

//pakker for SlaveData
use queues::*;

//pakker for Config:
use std::path::Path;

// Test for SlaveData modulen //
pub fn test_SlaveData() 
{
    let mut slave = master::SlaveData::new("1234".to_string());
    slave.buffer.size();
    slave.buffer.add(2);
    println!("buffer is empty: {}", slave.is_empty());
    let a = slave.buffer.remove().unwrap();
    println!("popped element {}", a);
    println!("buffer is empty: {}", slave.is_empty());
    slave.print();
}


// test for Config
pub fn test_Config() {
    let path = Path::new("config.json");
    let conf = config::Config(&path);
    conf.print();
}