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
            let listener: TcpListener = TcpListener::bind(config.elevator_ip_list[0].to_string() + ":" + &config.backup_port.to_string()).expect("Failed to bind");
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
                        spawn(|| handle_master_connection(stream, master_to_backup_tx, backup_to_master_rx));
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

    fn recieve_data_from_master(&mut self){
        
        match self.master_channels.1.try_recv(){
            Ok(message) => {
                println!("[BACKUP]\tRecieved message from master: {:#?}", message);
                match message{
                    Message::BackUp(data) => {
                        self.orders = data;
                    }
                    Message::Idle(a) => {
                        println!("[BACKUP]\tMaster is idle");
                    }
                    _ => {} // Do nothing for other types of incoming messages.
                }
            }
            Err(_) => {}
        }
    }

    pub fn backup_loop(&mut self){
        loop{
            self.recieve_data_from_master();
        }
    }
}


fn handle_master_connection(
    mut stream: TcpStream,
    master_to_backup_tx: cbc::Sender<Message>,
    backup_to_master_rx: cbc::Receiver<Message>,
)
{
    let mut encoded = [0; 1024];
    loop{

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
                break;
            }
        }

        match backup_to_master_rx.try_recv(){
            Ok(message) => {
                let encoded = bincode::serialize(&message).expect("Failed to serialize message");
                stream.write(&encoded).expect("Failed to write to stream");
            }
            Err(_) => {}
        }
    }
}