use bincode;
use crossbeam_channel as cbc;
use debug_print::debug_println as dprintln;
use std::fmt::{Display as FmtDisplay, Formatter as FmtFormatter, Result as FmtResult};
use std::io::Error;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;

use crate::config::{Config, BUFFER_SIZE};
use crate::io_datastructures::{ErrorState, Message, OrderRequests};
use crate::heartbeat;
use network_rust::udpnet;

/// Struck for Backup unit.
pub struct Backup {
    orders: OrderRequests,
    master_to_backup_rx: cbc::Receiver<Message>,
    heartbeat_rx: cbc::Receiver<udpnet::peers::PeerUpdate>,
}

impl Backup {
    /// Initilize a new Backup.
    /// Will loop until a master connection is established.
    /// Returns a backup unit with a master connected.
    pub fn init(config: &Config) -> Result<Backup, Error> {
        dprintln!("[BACKUP]\tInitializing backup");

        loop {
            // listen for all incoming connection on ip adress 0.0.0.0:backupPort.
            let listener: TcpListener =
                TcpListener::bind("0.0.0.0".to_string() + ":" + &config.backup_port.to_string())?;
            for stream in listener.incoming() {
                // Connects to one master only
                match stream {
                    Ok(stream) => {
                        let (master_to_backup_tx, master_to_backup_rx) =
                            cbc::unbounded::<Message>();
                        let (heart_update_tx, heart_update_rx) = cbc::unbounded::<udpnet::peers::PeerUpdate>();
                        heartbeat::recieve_online_statuses(heart_update_tx, config.heartbeat_port);
                        heartbeat::send_alive("backup".to_string(),config.heartbeat_port); 
                        spawn_thread_for_master_connection(stream, master_to_backup_tx);

                        let backup = Backup {
                            orders: OrderRequests::init(),
                            master_to_backup_rx,
                            heartbeat_rx: heart_update_rx,
                        };

                        dprintln!("[BACKUP]\tConnected to master");
                        return Ok(backup);
                    }
                    Err(e) => {
                        dprintln!("[BACKUP]\tError:\t{}", e);
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
                        Message::Backup(data) => {
                            self.orders = data;
                            dprintln!("[BACKUP]\tUpdated orders: {:#?}", self.orders);
                        }

                        // We asume that most errors ocure becouse of error in the master, so we start a new master.
                        // Worst case is we start a second master, but this will give an error so the master
                        // will not fully initialize, and instead start a new backup anyway
                        Message::Error(ErrorState::Network) => {
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
            match (self.heartbeat_rx.try_recv()) {
                Ok(msg)=>{
                    for ip in msg.lost{
                        if ip=="Master".to_string(){
                            println!("Master disconected");
                            return Ok(self.orders.clone());
                        }
                    }
                }
                Err(_e)=>{} //No update yet
            } 
        }
    }
}
impl FmtDisplay for Backup {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f,
            "Backup:\n\
            \tStored orders:\t{}\n\
            \tMaster channel:{:?}",
            self.orders, self.master_to_backup_rx
        )
    }
}

/// Spawn a new thread that will read from the TcpStream and send the message to the master_to_backup_tx channel.
fn spawn_thread_for_master_connection(
    mut stream: TcpStream,
    master_to_backup_tx: cbc::Sender<Message>,
) {
    spawn(move || {
        // TTL is set to 3 to prevent packets from being forwarded to other networks
        stream.set_ttl(3).expect("Failed to set TTL on stream");
        stream
            .set_nodelay(true)
            .expect("Failed to set nodelay on stream");
        stream
            .set_nonblocking(true)
            .expect("Failed to set non-blocking mode on stream");

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
                            Err(_e) => {
                                dprintln!("[BACKUP]\tFailed to send message to backup: {}", e);
                            }
                        }
                        break;
                    }
                }
            }
        }
    });
}
