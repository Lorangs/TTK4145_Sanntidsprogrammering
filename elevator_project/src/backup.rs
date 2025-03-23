use crossbeam_channel as cbc;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;

use crate::config::Config;
use crate::master::MasterQueues;
use crate::tcp::{Message, ErrorState};

pub struct Backup {
    orders: MasterQueues,
    master_to_backup_rx: cbc::Receiver<Message>,
}

impl Backup {
    // Loops unitl it connects to a master
    pub fn init(config: &Config) -> Backup {
        println!("[BACKUP]\tInitializing backup");

        loop {
            let listener: TcpListener = TcpListener::bind("0.0.0.0".to_string() + ":" + &config.backup_port.to_string())
                    .expect("Failed to bind");
            for stream in listener.incoming() {
                // Connects to one master only
                match stream {
                    Ok(stream) => {
                        let (master_to_backup_tx, master_to_backup_rx) = cbc::unbounded::<Message>();

                        let backup = Backup {
                            orders: MasterQueues::init(),
                            master_to_backup_rx: master_to_backup_rx,
                        };

                        let tcp_timeout_ms = config.tcp_timeout_ms.clone();
                        spawn(move || {
                            handle_master_connection(stream, master_to_backup_tx, tcp_timeout_ms)
                        });
                        println!("[BACKUP]\tConnected to master");
                        return backup;
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        sleep(Duration::from_secs(2));
                    }
                }
            }
        }
    }

    // Updates backup orders and returns them if master disconnects
    // Ned to handle the case where the backup recieves a message but dont update the orders. May need to be handled in both backup and master.
    pub fn backup_loop(&mut self) -> Result<MasterQueues, cbc::RecvError> {
        loop {
            match self.master_to_backup_rx.recv() {
                Ok(message) => {
                    match message {
                        Message::Backup(data) => {
                            self.orders = data;
                            println!("[BACKUP]\tUpdated orders: {:#?}", self.orders);
                        }
                        Message::Error(ErrorState::Network) => {
                            println!("[BACKUP]\tMaster disconnected");
                            return Ok(self.orders.clone());
                        }
                        _ => {} // Do nothing for other types of incoming messages.
                    }
                }
                Err(cbc::RecvError) => {
                    println!("[BACKUP]\tMaster disconnected");

                    // try sending error state to master so that master can initilize an other backup.
                    // if not possible, return orders and inititize self as new master.

                    return Ok(self.orders.clone());
                }
            }
        }
    }
}


// Handles incoming messages from master. 
fn handle_master_connection(
    mut stream: TcpStream,
    master_to_backup_tx: cbc::Sender<Message>,
    tcp_timeout_ms: u64, // Not used. Need to bee a Duration to be passed to stream.set_read_timeout()
) //-> Result<(), cbc::RecvError>
{
    let mut encoded = [0; 1024];
    stream
        .set_read_timeout(Some(Duration::from_millis(tcp_timeout_ms)))
        .expect("Failed to set read timeout");

    loop {
        match stream.read(&mut encoded) {
            Ok(size) => {
                if size > 0 {
                    let received: Message =
                        bincode::deserialize(&encoded[..size]).expect("Failed to deserialize message");
                    master_to_backup_tx.send(received).unwrap();
                }
                else{ //e dinna so blir utført vist eg drepe master
                    println!("[BACKUP]\t Lost conection to master");
                    let message = Message::Error(ErrorState::Network);
                    master_to_backup_tx.send(message).unwrap();
                    break;
                }
            }
            Err(e) => { // då kan vi nokk forenkle eller fjerne dinna litt trudde den kom til å fange opp disconecten
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    // No data available, continue the loop
                    //println!("[BACKUP]\tNo data available");
                    continue;
                } else {
                    // Connection lost or other error
                    println!("[BACKUP]\tError: {}", e);
                    let message = Message::Error(ErrorState::Network);
                    master_to_backup_tx.send(message).unwrap();
                    break;
                }
            }
        }
    }
}
