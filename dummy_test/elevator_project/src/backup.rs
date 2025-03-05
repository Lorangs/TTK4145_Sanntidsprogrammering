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
    master_to_backup_rx     : cbc::Receiver<Message>,
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
                        
                        let backup = Backup {
                            config              : config.clone(),
                            orders              : MasterQueues::init(),
                            master_to_backup_rx : master_to_backup_rx
                        };
                        
                        let tcp_timeout_ms = config.tcp_timeout_ms.clone();
                        spawn(move || handle_master_connection(stream, master_to_backup_tx, tcp_timeout_ms));
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

    pub fn backup_loop(&mut self) -> MasterQueues {
        loop {
            match self.master_to_backup_rx.recv() {
                Ok(message) => {
                    match message{
                        Message::Backup(data) => {
                            self.orders = data;
                            println!("[BACKUP]\tUpdated orders: {:#?}", self.orders);
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
    tcp_timeout_ms          : u64,        
) //-> Result<(), cbc::RecvError>
{
    let mut encoded = [0; 1024];
    loop{
        //stream.set_read_timeout(Some(Duration::from_millis(tcp_timeout_ms))).expect("Failed to set read timeout");
        match stream.read(&mut encoded){
            Ok(size) => {
                if size > 0 {
                    let recieved: Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                    master_to_backup_tx.send(recieved).unwrap();

                }
            }
            Err(e) => {
                println!("Error: {}", e);
                //return Err(cbc::RecvError);
            }
        }
    }
}