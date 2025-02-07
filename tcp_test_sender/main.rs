use std::net::TcpStream;
use std::io::prelude::*;
use serde::{Serialize, Deserialize};
use bincode;


#[derive(Serialize, Deserialize, Debug)]
pub enum Message{
    NewOrder(u8),
    OrderComplete(u8),
    Error(u8),
}


fn main() {

    let mut stream = match TcpStream::connect("10.22.128.63:7878") {
        Ok(stream) => stream,
        Err(e) => {
            println!("Failed to connect: {}", e);
            return;
        }
    };

    let buffer: Message = Message::NewOrder(3);
    let encode: Vec<u8> = bincode::serialize(&buffer).expect("Failed to serialize message");

    let _ = match stream.write(&encode) {
        Ok(_) => println!("Sent message"),
        Err(e) => println!("Failed to send message: {}", e),
    };
}