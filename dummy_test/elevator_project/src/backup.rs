use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;
use crossbeam_channel as cbc;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

use crate::tcp::{self, Message};
use crate::config::Config;
use crate::master::MasterQueues;


pub struct Backup{
    config                  : Config,   
    orders                  : MasterQueues,     
    master_channels         : (cbc::Sender<Message>, cbc::Receiver<Message>),
}

impl Backup{
    pub fn init(
        config  : &Config,
    ) -> Backup
    {
        println!("[BACKUP]\tInitializing backup");

        loop {  // loops unitl it connects to a master
            let listener: TcpListener = TcpListener::bind("0.0.0.0".to_string() + ":" + &config.backup_port.to_string()).expect("Failed to bind");
            for stream in  listener.incoming(){   // Connects to one master only
                match stream{
                    Ok(stream) => {
                        let (master_to_backup_tx, master_to_backup_rx) = cbc::unbounded::<Message>();
                        let (backup_to_master_tx, backup_to_master_rx) = cbc::unbounded::<Message>();
                        let backup = Backup {
                            config              : config.clone(),
                            orders              : MasterQueues::init(),
                            master_channels     : (backup_to_master_tx, master_to_backup_rx),
                        };
                        
                        println!("[BACKUP]\tConnected to master: {}", stream.peer_addr().unwrap());
                        let tcp_timeout_ms = config.tcp_timeout_ms.clone();
                        spawn(move || handle_master_connection(stream, master_to_backup_tx, backup_to_master_rx, tcp_timeout_ms));
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

    pub fn backup_loop(&mut self) -> MasterQueues {
        loop {
            match self.master_channels.1.recv() {
                Ok(message) => {
                    println!("[BACKUP]\tRecieved message from master: {:#?}", message);
                    match message{
                        Message::BackUp(data) => {
                            self.orders = data;
                        }
                        _ => {} // Do nothing for other types of incoming messages.
                    }
                }
                Err(cbc::RecvError) => {
                    println!("[BACKUP]\tMaster disconnected");
                    return self.orders.clone();


                }
            }
        }
    }
}


fn handle_master_connection
(
    mut stream              : TcpStream,
    master_to_backup_tx     : cbc::Sender<Message>,
    backup_to_master_rx     : cbc::Receiver<Message>,
    tcp_timeout_ms          : u64,        
) -> Result<(), cbc::RecvError>
{
    let mut encoded = [0; 1024];
    loop{
        stream.set_read_timeout(Some(Duration::from_millis(tcp_timeout_ms))).expect("Failed to set read timeout");
        match stream.read(&mut encoded){
            Ok(size) => {
                if size > 0 {
                    let recieved: Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                    println!("[BACKUP]\tRecieved message from master: {:#?}", recieved);
                    master_to_backup_tx.send(recieved).unwrap();

                }
            }
            Err(e) => {
                println!("Error: {}", e);
                return Err(cbc::RecvError);
            }
        }
    }
}