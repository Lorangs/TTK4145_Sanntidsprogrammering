use std::thread::{spawn, sleep, Builder};
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::net::{Incoming, TcpListener, TcpStream};
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};
use std::string::String;
use crate::{config, inputs, slave, tcp};

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
        if self.cab_queues.len() > slave_num as usize {
            self.cab_queues[slave_num as usize].push_back(floor);
        } else {
            println!("Error: Slave queue index {} is out of bounds!", slave_num);
        }
    }
}


// Master implementation
#[derive(Debug)]
pub struct Master {
    pub config              : config::Config,                                           // Config struct                                 
    slaves_ip               : Vec<String>,                                              // Vector of slaves IP addresses
    backup_ip               : String,                                                   // IP address of backup
    order_queues            : MasterQueues,                                             // Vector of slaves order queues
    incoming_clients_rx     : cbc::Receiver<TcpStream>,                                               // Incoming connections
    slave_sockets           : Vec<TcpStream>,                                 // Vector of slave sockets
    slave_channels          : Vec<cbc::Receiver<tcp::Message>>,                              // Vector of slave channels
    //backup_socket           : TcpStream,                                              // Backup socket
    //slaves_rx               : Vec<cbc::Receiver<tcp::Message>>,                 // Vector of slaves message receivers
}


impl Master {
    pub fn init(
        config              : &config::Config,
        master_ip           : &String
    ) -> Result<Master, String> 
    {
        let backup_ip       : String            = match config.elevator_ip_list.iter().find(|&ip| *ip != *master_ip) 
                                                        {
                                                            Some(ip) => ip.to_string() + ":" + &config.backup_port.to_string(),
                                                            None => return Err("No valid backup IP found".to_string())
                                                        };
      


        // Create channel for incoming connections                                                                                                  
        let (incoming_clients_tx, incoming_clients_rx) = cbc::unbounded();

        let slave_sockets  : Vec<TcpStream> = Vec::new();
        let slave_channels : Vec<cbc::Receiver<tcp::Message>> = Vec::new();
        
        let master = Master {
            config                  : config.clone(),
            backup_ip               : backup_ip,                              // IP address of backup
            slaves_ip               : config.elevator_ip_list.clone(),        // Vector of slaves IP addresses                 
            order_queues            : MasterQueues::init(),                   // Vector of slaves order queues
            incoming_clients_rx     : incoming_clients_rx,                    // Incoming connections
            slave_sockets           : slave_sockets,               // Vector of slave sockets  TODO: Fix size
            slave_channels          : slave_channels,                         // Vector of slave channels TODO: Fix size
            //backup_socket           : backup_socket,                        // Backup socket
        }; 

        // Thread for listening for new slave connections
        let slave_port = config.slave_port.to_string();
        Builder::new().name("Incomig Connections".to_string()).spawn(move || {
            match inputs::listen_for_new_connection(&slave_port) {
                Some(stream) => {
                    incoming_clients_tx.send(stream).unwrap();
                },
                None => { println!("[MASTER]\tFailed to establish connection"); }
            };      
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
                println!("[MASTER]\tRemoving slave from list");
                                
            }
        }   
    }


    // Tanken er å teste konnekjsonen til slavene hvert x sekund. Fjernes dersom de ikke svarer.
    fn test_connection_to_slaves(&mut self) {
        let message = tcp::Message::ConnectionTest;
        let encoded = bincode::serialize(&message).unwrap();
        let mut indices_to_remove: Vec<usize> = Vec::new();

        for (index, mut slave_socket) in self.slave_sockets.iter().enumerate() {
            match slave_socket.write(&encoded) {
                Ok(_) => {
                    // println!("[MASTER]\tConnection test sent to slave:\t{}", slave_socket.peer_addr().unwrap());
                }
                Err(e) => {
                    println!("[MASTER]\tFailed to send connection test to slave:\t{}", e);
                    println!("[MASTER]\tRemoving slave from list"); 
                    indices_to_remove.push(index);
                }       
            }
        }

        // remove slaves that did not respond
        if indices_to_remove.len() > 0 {
            for index in indices_to_remove {
                self.slave_sockets.remove(index);
                self.slave_channels.remove(index);
            }
        }
    }

    // Implementer samme funksjonalitet for backup. Enten i samme func eller separat (duplisert kode :())
    
    pub fn master_loop(&mut self) {
        loop 
        {    
            let mut select = cbc::Select::new();
            let mut handles: Vec<usize> = Vec::new();
            
            // Register the incoming clients receiver
            handles.push(select.recv(&self.incoming_clients_rx));
            
            // Register the slave channels
            for rx in &self.slave_channels 
            {
                handles.push(select.recv(rx));
            }
            
            // Wait for any of the receivers to receive a message
            let operator = select.select();
            let index = operator.index();
            match index {
                // Handle incoming client connections
                0 => {
                    let stream = operator.recv(&self.incoming_clients_rx).unwrap();
                    let stream_clone = stream.try_clone().expect("Failed to clone stream");
                    let slave_rx = inputs::master_read_from_clients(stream_clone, self.config.input_poll_rate_ms);
                    self.slave_channels.push(slave_rx);
                    self.slave_sockets.push(stream);
                    
                }

                // Handle messages from slave channels
                _ => {
                    let message = operator.recv(&self.slave_channels[index - 1]).unwrap();  
                    match message          
                    {
                        tcp::Message::NewOrder(floor, button_type) => {
                            match button_type {
                                driver_rust::elevio::elev::HALL_UP => {
                                    self.order_queues.add_to_hall_queue(floor, driver_rust::elevio::elev::HALL_UP);
                                }
                                driver_rust::elevio::elev::HALL_DOWN => {
                                    self.order_queues.add_to_hall_queue(floor, driver_rust::elevio::elev::HALL_DOWN);
                                }
                                driver_rust::elevio::elev::CAB => {
                                    self.order_queues.add_to_cab_queue(0, floor);
                                }
                                _ => {}
                            }
                        }

                        tcp::Message::OrderComplete => {
                            println!("[MASTER]\tOrder complete");
                            // TODO: Implement functionality to handle order completion
                        }

                        tcp::Message::Error(_) => {
                            println!("[MASTER]\tReceived error message from slave");
                            // TODO: Implement functionality to handle errors from slave
                        }

                        tcp::Message::ConnectionTest => {}
                    }
                }
            }
        }
    }
}




