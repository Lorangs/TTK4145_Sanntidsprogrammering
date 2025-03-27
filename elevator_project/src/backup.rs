use bincode;
use crossbeam_channel as cbc;
use debug_print::debug_println as dprintln;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;
use std::io::Error;

use crate::config::{Config, BUFFER_SIZE};
use crate::master::OrderRequests;
use crate::io_datastructures::{ErrorState, Message};

pub struct Backup {
    orders: OrderRequests,
    master_to_backup_rx: cbc::Receiver<Message>,
}

impl Backup {
    // Loops unitl it connects to a master
    pub fn init(config: &Config) -> Result<Backup, Error> {
        dprintln!("[BACKUP]\tInitializing backup");

        loop {
            let listener: TcpListener =
                TcpListener::bind("0.0.0.0".to_string() + ":" + &config.backup_port.to_string())?;
            for stream in listener.incoming() {
                // Connects to one master only
                match stream {
                    Ok(stream) => {
                        let (master_to_backup_tx, master_to_backup_rx) =
                            cbc::unbounded::<Message>();

                        let backup = Backup {
                            orders: OrderRequests::init(),
                            master_to_backup_rx,
                        };

                        spawn(move || handle_master_connection(stream, master_to_backup_tx));
                        dprintln!("[BACKUP]\tConnected to master");
                        return Ok(backup)
                    }
                    Err(e) => {
                        dprintln!("Error: {}", e);
                        sleep(Duration::from_secs(2));
                    }
                }
            }
        }
    }

    // Updates backup orders and returns them if master disconnects
    // Updates backup orders and returns them if master disconnects
    pub fn backup_loop(&mut self) -> Result<OrderRequests, cbc::RecvError> {
        loop {
            match self.master_to_backup_rx.recv() {
                Ok(message) => {
                    match message {
                        Message::Backup(data) => {
                            self.orders = data;
                            dprintln!("[BACKUP]\tUpdated orders: {:#?}", self.orders);
                        }
                        Message::Error(ErrorState::Network) => {
                            //We asume that most errors ocure becouse of error in the master, so we start a new master. 
                            //Worst case is we start a second master, but this will give an error so the master will not fully initialize, and instead start a new backup anyway
                            dprintln!("[BACKUP]\tMaster disconnected");
                            return Ok(self.orders.clone());
                        }
                        _ => {} // Do nothing for other types of incoming messages.
                    }
                }
                Err(cbc::RecvError) => {
                    dprintln!("[BACKUP]\tMaster disconnected");
                    return Ok(self.orders.clone());
                }
            }
        }
    }
}


// Handles incoming messages from master.
fn handle_master_connection(mut stream: TcpStream, master_to_backup_tx: cbc::Sender<Message>)
{
    // TTL is set to 3 to prevent packets from being forwarded to other networks. Nodelay is set to true to disable Nagle's algorithm, witch reduces latency.
    stream.set_ttl(3).expect("Failed to set TTL");
    stream.set_nodelay(true).expect("Failed to set nodelay");
    stream.set_nonblocking(true).expect("Failed to set nonblocking");

    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];



    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size > 0 {
                    let msg: Message = bincode::deserialize::<Message>(&buffer[..size])
                        .expect("Failed to deserialize message");
                    dprintln!("[BACKUP]\tReceived message from master: {:#?}", msg);
                    match master_to_backup_tx.send(msg) {
                        Ok(_) => {}
                        Err(e) => {
                            dprintln!("[BACKUP]\tFailed to send message to backup: {}", e);
                            break;
                        }
                    }
                } else {
                    dprintln!("[BACKUP]\tLost conection to master");
                    let msg = Message::Error(ErrorState::Network);
                    match master_to_backup_tx.send(msg) {
                        Ok(_) => {}
                        Err(e) => {
                            dprintln!("[BACKUP]\tFailed to send message to backup: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    // No data available, continue the loop
                    continue;
                } else {
                    // Connection lost or other error
                    dprintln!("[BACKUP]\tError: {}", e);
                    let msg = Message::Error(ErrorState::Network);
                    match master_to_backup_tx.send(msg) {
                        Ok(_) => {}
                        Err(e) => {
                            dprintln!("[BACKUP]\tFailed to send message to backup: {}", e);
                        }
                    }
                    break;
                }
            }
        }
    }
}
