use crossbeam_channel as cbc;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;
use bincode;
use crate::config::{Config, BUFFER_SIZE};
use crate::master::OrderRequests;
use crate::tcp;
use debug_print::debug_println as dprintln;

pub struct Backup {
    orders: OrderRequests,
    master_to_backup_rx: cbc::Receiver<tcp::Message>,
}

impl Backup {

    /// Initilize a backup unit. 
    /// Will loop until a master connects to the backup.
    pub fn init(config: &Config) -> Backup {
        dprintln!("[BACKUP]\tInitializing backup");

        loop {
            let listener: TcpListener = TcpListener::bind("0.0.0.0".to_string() + ":" + &config.backup_port.to_string())
                    .expect("Failed to bind");
            for stream in listener.incoming() {
                // Connects to one master only
                match stream {
                    Ok(stream) => {
                        let (master_to_backup_tx, master_to_backup_rx) = cbc::unbounded::<tcp::Message>();

                        let backup = Backup {
                            orders: OrderRequests::init(),
                            master_to_backup_rx: master_to_backup_rx,
                        };

                        spawn_thread_for_master_connection(stream, master_to_backup_tx);
        
                        dprintln!("[BACKUP]\tConnected to master");
                        return backup;
                    }
                    Err(e) => {
                        dprintln!("Error: {}", e);
                        sleep(Duration::from_secs(2));
                    }
                }
            }
        }
    }

    /// Updates backup and returns the stored oreder_requests if master disconnects
    pub fn backup_loop(&mut self) -> Result<OrderRequests, cbc::RecvError> {
        loop {
            match self.master_to_backup_rx.recv() {
                Ok(message) => {
                    match message {
                        tcp::Message::Backup(data) => {
                            self.orders = data;
                            dprintln!("[BACKUP]\tUpdated orders: {:#?}", self.orders);
                        }
                        tcp::Message::Error(tcp::ErrorState::Network) => {
                            dprintln!("[BACKUP]\tMaster disconnected");
                            return Ok(self.orders.clone());
                        }
                        _ => {} // Do nothing for other types of incoming messages.
                    }
                }
                Err(cbc::RecvError) => {
                    dprintln!("[BACKUP]\tMaster disconnected");

                    // try sending error state to master so that master can initilize an other backup.
                    // if not possible, return orders and inititize self as new master.

                    return Ok(self.orders.clone());
                }
            }
        }
    }
}


/// Spawn a new thread that will read from the TcpStream and send the message to the master_to_backup_tx channel.
fn spawn_thread_for_master_connection(
    mut stream: TcpStream,
    master_to_backup_tx: cbc::Sender<tcp::Message>,
) 
{
    spawn ( move || {

        // TTL is set to 3 to prevent packets from being forwarded to other networks
        stream.set_ttl(3).expect("Failed to set TTL on stream");
        stream.set_nodelay(true).expect("Failed to set nodelay on stream");
        stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
        
        let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

        loop {
            match stream.read(&mut buffer) {
                Ok(size) => {
                    if size > 0 {
                        let msg: tcp::Message = bincode::deserialize::<tcp::Message>(&buffer[..size]).expect("Failed to deserialize message");
                        dprintln!("[BACKUP]\tReceived message from master: {:#?}", msg);
                        master_to_backup_tx.send(msg).unwrap();
                    }
                    else{ //e dinna so blir utført vist eg drepe master
                        dprintln!("[BACKUP]\tLost conection to master");
                        let msg = tcp::Message::Error(tcp::ErrorState::Network);
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
                        dprintln!("[BACKUP]\tError: {}", e);
                        let msg = tcp::Message::Error(tcp::ErrorState::Network);
                        master_to_backup_tx.send(msg).unwrap();
                        break;
                    }
                }
            }
        }
    });
}
