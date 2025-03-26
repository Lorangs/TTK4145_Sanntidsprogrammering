use crossbeam_channel as cbc;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;
use bincode;

use crate::config::Config;
use crate::master::OrderRequests;
use crate::tcp::{Message, ErrorState};

pub struct Backup {
    orders: OrderRequests,
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
                            orders: OrderRequests::init(),
                            master_to_backup_rx: master_to_backup_rx,
                        };

                        spawn(move || {
                            handle_master_connection(stream, master_to_backup_tx)
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
    pub fn backup_loop(&mut self) -> Result<OrderRequests, cbc::RecvError> {
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
) //-> Result<(), cbc::RecvError>
{

    // TTL is set to 3 to prevent packets from being forwarded to other networks
    stream.set_ttl(3).expect("Failed to set TTL on stream");
    stream.set_nodelay(true).expect("Failed to set nodelay on stream");
    stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
    
    let mut buffer: [u8; 64] = [0; 64];

    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size > 0 {
                    let msg: Message = bincode::deserialize::<Message>(&buffer).expect("Failed to deserialize message");
                    println!("[BACKUP]\tReceived message from master: {:#?}", msg);
                    master_to_backup_tx.send(msg).unwrap();
                }
                else{ //e dinna so blir utført vist eg drepe master
                    println!("[BACKUP]\tLost conection to master");
                    let msg = Message::Error(ErrorState::Network);
                    master_to_backup_tx.send(msg).unwrap();
                    break;
                }
            }
            Err(e) => { // då kan vi nokk forenkle eller fjerne dinna litt trudde den kom til å fange opp disconecten
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    // No data available, continue the loop
                    continue;
                } else {
                    // Connection lost or other error
                    println!("[BACKUP]\tError: {}", e);
                    let msg = Message::Error(ErrorState::Network);
                    master_to_backup_tx.send(msg).unwrap();
                    break;
                }
            }
        }
    }
}
