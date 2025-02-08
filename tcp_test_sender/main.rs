use std::net::TcpStream;
use std::io::prelude::*;
use serde::{Serialize, Deserialize};
use bincode;
use crossbeam_channel as cbc;
use std::thread::{spawn, sleep};
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


struct Slave {
    master_socket       : TcpStream,
    master_message      : (cbc::Sender<Message>, cbc::Receiver<Message>),
}

impl Slave {
    fn init(master_ip: String) -> Slave {
        let mut slave = Self {
            master_socket                       : TcpStream::connect(master_ip).expect("Failed to connect to master"),
            master_message                      : cbc::unbounded::<Message>(),
        };

        slave.spawn_thread_for_incoming_messages();
        slave.send_new_cab_order(3);
        return slave;
    }
    
    fn spawn_thread_for_incoming_messages(&self) {
        let mut master_socket_clone = self.master_socket.try_clone().expect("Failed to clone socket"); 
        let tx = self.master_message.0.clone();
        
        spawn (move || {
            let mut encoded = [0; 1024];
            loop{
                match master_socket_clone.read(&mut encoded) {
                    Ok(size) => {
                        if size > 0 {
                            let message: Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                            println!("[SLAVE]\tReceived message from master: {:#?}", message);
                            tx.send(message).unwrap();
                        }
                    }
                    Err(e) => {
                        println!("[SLAVE]\tFailed to read from stream: {}", e);
                        return e;
                    }
                }            
                sleep(Duration::from_millis(10));
            }
        });
    }

    pub fn send_new_cab_order(&mut self, cab_order: u8) {    
        let message = Message::NewOrder(cab_order);
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_)    => println!("[SLAVE]\tSent cab order: {}", cab_order),
            Err(e)   => println!("[SLAVE]\tFailed to send cab order: {}", e),
        }
    }
}
    



fn main() {
    let mut slave = Slave::init("localhost:3333".to_string());
    loop {
        slave.send_new_cab_order(3);
        sleep(Duration::from_secs(5));
    }
}