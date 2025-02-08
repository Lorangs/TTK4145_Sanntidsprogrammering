use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use bincode;
use std::thread::sleep;
use std::time::Duration;
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message{
    NewOrder(u8),
    OrderComplete(u8),
    Error(u8),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::NewOrder(id) => write!(f, "New order: {}", id),
            Message::OrderComplete(id) => write!(f, "Order complete: {}", id),
            Message::Error(id) => write!(f, "Error: {}", id),
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut encoded = [0; 1024];
    let message = Message::OrderComplete(1);
    let message_encoded = bincode::serialize(&message).unwrap();

    loop {
        match stream.read(&mut encoded) {
            Ok(0) => {
                println!("Client disconnected");
            }
            Ok(n) => {
                let message: Message = bincode::deserialize(&encoded[..n]).unwrap();
                println!("Received: {}", &message);
                stream.write(&message_encoded).unwrap();

            }
            Err(e) => {
                eprintln!("Failed to read from connection: {}", e);
            }
        }
        sleep(Duration::from_secs(1));
    }
}

fn main() {
    let listener = match TcpListener::bind("localhost:3333"){
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind: {}", e);
            return;
        }
    };

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                eprintln!("Failed to establish connection: {}", e);
            }
        }
    }
}