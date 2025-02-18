#![allow(warnings)]

use std::thread::{spawn, sleep, Builder};
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::net::{Incoming, TcpListener, TcpStream};
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};
use std::string::String;
use crate::config::Config;
use crate::inputs;
use crate::slave;
use crate::tcp;

use crossbeam_channel as cbc;

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterQueues {
    pub hall_queue: VecDeque<(u8, u8)>,     // (floor, button_type) for external hall calls.
    pub cab_queues: Vec<VecDeque<u8>>,      // Vector of slave queues for internal cab calls.  ref driver_rust::elevio::poll::CallButton
}


impl FmtDisplay for MasterQueues {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f, 
            "Hall queue: {:?}\n\
            Cab queues: {:?}", 
            self.hall_queue, 
            self.cab_queues)
    }
}


impl MasterQueues {
    pub fn init() -> MasterQueues {
        let hall_queue      : VecDeque<(u8, u8)>    = VecDeque::new();
        let cab_queues      : Vec<VecDeque<u8>>     = Vec::new();
        
        MasterQueues {
            hall_queue,
            cab_queues,
        }
    }

    pub fn add_to_hall_queue(&mut self, floor: u8, direction: u8) {
        self.hall_queue.push_back((floor, direction));
    }

    pub fn add_to_cab_queue(&mut self, slave_num: u8, floor: u8) {
        //TODO
    }
}


// Master implementation
#[derive(Debug)]
pub struct Master {
    pub config              : Config,                                           // Config struct                                 
    slaves_ip               : Vec<String>,                                              // Vector of slaves IP addresses
    backup_ip               : String,                                                   // IP address of backup
    order_queues            : MasterQueues,                                             // Vector of slaves order queues
    incoming_clients_rx     : cbc::Receiver<TcpStream>,                                               // Incoming connections
    slave_sockets           : Vec<Option<TcpStream>>,                                 // Vector of slave sockets
    slave_channels          : Vec<cbc::Receiver<tcp::Message>>,                              // Vector of slave channels
    //backup_socket           : TcpStream,                                              // Backup socket
    //slaves_rx               : Vec<cbc::Receiver<tcp::Message>>,                 // Vector of slaves message receivers
}


impl Master {
    pub fn init(
        config              : &Config,
        master_ip           : &String
    ) -> Result<Master, String> {

        let conf            : Config    = config.clone();
        let backup_ip       : String            = match config.elevator_ip_list.iter().find(|&ip| *ip != *master_ip) 
                                                        {
                                                            Some(ip) => ip.to_string() + ":" + &config.backup_port.to_string(),
                                                            None => return Err("No valid backup IP found".to_string())
                                                        };
      


/* 
        // connect to backup. Will not continue until connection is established
        let mut backup_socket : Option<TcpStream> = None;
        while backup_socket.is_none() {
            backup_socket = listen_for_new_connection(&config.backup_port.to_string()) 
        } */


        // Create channel for incoming connections                                                                                                  
        let (incoming_conn_tx, incoming_conn_rx) = cbc::unbounded();

        let mut slave_channels : Vec<cbc::Receiver<tcp::Message>> = Vec::new();
        


        let master = Master {
            config                  : config.clone(),
            backup_ip               : backup_ip,                              // IP address of backup
            slaves_ip               : config.elevator_ip_list.clone(),                // Vector of slaves IP addresses                 
            order_queues            : MasterQueues::init(),                   // Vector of slaves order queues
            incoming_clients_rx     : incoming_conn_rx,                       // Incoming connections
            slave_sockets           : vec![ None, None, None ],               // Vector of slave sockets  TODO: Fix size
            slave_channels          : slave_channels,                                   // Vector of slave channels TODO: Fix size
            //backup_socket           : backup_socket,                          // Backup socket
        }; 

        

        // Thread for listening for new slave connections
        let slave_port = config.slave_port.to_string();
        Builder::new().name("Incomig Connections".to_string()).spawn(move || {
            // skal det være loop her eller ikke?? må teste
            let incoming_tcp_slave = inputs::listen_for_new_connection(&slave_port).unwrap();
            incoming_conn_tx.send(incoming_tcp_slave).unwrap();        
        }).unwrap();

        Ok(master)
    
    }


    // Vurdere å flytte til inputs eller inne i handle_clients. Problem: Lese fra kø fra annen tråd. 
    fn send_order_to_slave(&self, mut slave_socket: TcpStream, nxt_order: u8) {
        let message = tcp::Message::NewOrder(nxt_order, 0); 
        let encoded = bincode::serialize(&message).unwrap();
        match slave_socket.write(&encoded) {
            Ok(_) => {
                println!("[MASTER]\tOrder sent to slave:\t{}", slave_socket.peer_addr().unwrap());
            }
            Err(e) => {
                println!("[MASTER]\tFailed to send order to slave:\t{}", e);
            }
        }
        
    }

    // Implementer samme funksjonalitet for backup. Enten i samme func eller separat (duplisert kode :())
    
    pub fn master_loop(&mut self) {
        let mut i=0;
        loop{
            println!("Master loop");
            sleep(std::time::Duration::from_secs(1));
            i+=1;
            if i==5 {
                break;
            }
        }
    }


}
