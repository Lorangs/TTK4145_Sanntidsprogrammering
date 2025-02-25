#![allow(warnings)]

use std::thread::{spawn, sleep, Builder};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{Incoming, TcpListener, TcpStream};
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::collections::VecDeque;
use std::string::String;
use std::sync::{Arc, Mutex};

use serde::{Serialize, Deserialize};
use crossbeam_channel as cbc;

use crate::config::Config;
use crate::inputs::{self, listen_for_new_connection};
use crate::slave;
use crate::tcp::{self, Message};

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
        self.cab_queues[slave_num as usize].push_back(floor);
    }

    pub fn get_next_order(&mut self) -> (Option<(u8, u8)>) {
        //reryrner fra cab que vist den e tom, returner fra hall que
        self.hall_queue.pop_front()
    }
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


// Master implementation
#[derive(Debug)]
pub struct Master {
    pub config              : Config,                                                   // Config struct                                 
    slaves_ip               : Vec<String>,                                              // Vector of slaves IP addresses
    backup_ip               : String,                                                   // IP address of backup
    pub order_queues            : MasterQueues,                                             // Vector of slaves order queues
    incoming_clients_rx     : cbc::Receiver<TcpStream>,                                 // Incoming connections
    slave_channels          : Arc<Mutex<Vec<(cbc::Receiver<Message>, cbc::Sender<Message>)>>>,      // Vector of slave channels. Sygt som fy
    //backup_socket           : TcpStream,                                              // Backup socket
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
        let mut slave_channels : Arc<Mutex<Vec<(cbc::Receiver<Message>, cbc::Sender<Message>)>>> = Arc::new(Mutex::new(Vec::new()));

        let master = Master {
            config                  : config.clone(),
            backup_ip               : backup_ip,                              
            slaves_ip               : config.elevator_ip_list.clone(),                         
            order_queues            : MasterQueues::init(),                   
            incoming_clients_rx     : incoming_conn_rx,                       
            slave_channels          : slave_channels,                                   
            //backup_socket           : backup_socket,                          
        }; 

        

        // Thread for listening for new slave connections
    
        let slave_port = config.slave_port.to_string();

        
        let slave_channels_clone = Arc::clone(&master.slave_channels);
        spawn (move || {
            let listener  = TcpListener::bind("0.0.0.0".to_string() + ":" + slave_port.as_str()).expect("Failed to bind");
            
            for stream in listener.incoming() {
                let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded();
                let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded();
                let mut locked_channel = slave_channels_clone.lock().unwrap();
                locked_channel.push((slave_to_master_rx, master_to_slave_tx));
                drop(locked_channel);
                println!("[MASTER]\tGot new stream");
                
                match stream {
                    Ok(stream) => {
                        println!("[MASTER]\tNew slave connection established: {}", stream.peer_addr().unwrap());
                        spawn(|| handle_slave_connection(stream, slave_to_master_tx, master_to_slave_rx));
                    }
                    Err(e) => {
                        eprintln!("[MASTER]\tFailed to establish connection to slave: {}", e);
                    }
                }

            }
        });

        
        // Builder::new().name("Incomig Connections".to_string()).spawn(move || {
        //     // skal det være loop her eller ikke?? må teste
        //     let incoming_tcp_slave: TcpStream = inputs::listen_for_new_connection(&slave_port).unwrap();
        //     master.slave_sockets[0] = Some(incoming_tcp_slave.try_clone().unwrap());
        //     //incoming_conn_tx.send(incoming_tcp_slave).unwrap();        
        // }).unwrap();

        Ok(master)
    
    }


    // Vurdere å flytte til inputs eller inne i handle_clients. Problem: Lese fra kø fra annen tråd. 
    fn send_order_to_slave(&self, nxt_order: u8, slave_number: u8) {
        let message = Message::NewOrder(nxt_order, 0); 
        let mut locked_channels = self.slave_channels.lock().unwrap();
        locked_channels[slave_number as usize].1.send(message).unwrap(); // Skriv om for bedre lesbarehet enn 0 og 1 
        println!("[MASTER]\tSent order to slave:"); 

    }


    fn recive_order_from_slave(&mut self, slave_number: u8) {
        //ha en cbc selekt hær som lese kanala til fleire slava og håndtera det?
        let mut locked_channels = self.slave_channels.lock().unwrap();
        match locked_channels[slave_number as usize].0.try_recv() {
            Ok(message) => {
                println!("[MASTER]\tResived message from slave: {:#?}", message);
                match message {
                    Message::NewOrder(floor, button_type) => {
                        if button_type == 2 {
                            self.order_queues.add_to_cab_queue(slave_number, floor);
                            println!("[MASTER]\tAdded order to cab queue");
                        } else {
                            self.order_queues.add_to_hall_queue(floor, button_type);
                            println!("[MASTER]\tAdded order to hall queue");
                        }
                    }
                    _ => {
                        eprintln!("[MASTER]\tReceived unexpected message from slave {:#?}", message);
                    }
                }
                               
            }
            Err(_) => {
                //eprintln!("[MASTER]\tFailed to read from master_to_slave_rx channel");
                
            }
        }   
    }

    // Implementer samme funksjonalitet for backup. Enten i samme func eller separat (duplisert kode :())
    
    pub fn master_loop(&mut self) {
        std::thread::sleep(std::time::Duration::from_secs(3));

        let start_time = std::time::Instant::now();
        let duration = std::time::Duration::from_secs(15);
    
        while start_time.elapsed() < duration {
            
            self.recive_order_from_slave(0);

            let order = self.order_queues.get_next_order();
            if order != None {
                println!("[MASTER]\tGot order from queue");
            
                self.send_order_to_slave(order.unwrap().0, 0); 
            }           
            // Optional: Add a very small sleep to avoid consuming 100% CPU
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

}





// Implementer funksjon for å håndtere meldinger fra slave
fn handle_slave_connection(mut stream: TcpStream, slave_to_master_tx: cbc::Sender<tcp::Message>, master_to_slave_rx: cbc::Receiver<tcp::Message>) {
    let mut buffer = [0; 1024];
    loop {
        stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size > 0 {
                    let recieved: tcp::Message = bincode::deserialize(&buffer[..size]).expect("Failed to deserialize message from slave");
                    println!("[MASTER]\tReceived message from slave: {:#?}", recieved);
                    slave_to_master_tx.send(recieved).unwrap();
                }

                //Implement message handlie new, I hesitate to overwhelm you with additional information. But, you could also implement a single-threaded TCP server that does something similar using tokio - then you could replace this with Rc<RefCell<Vec<String>>, which is an analogous construct for single-threaded scenarios.ng here
            }
            Err(e) => {
                //eprintln!("[MASTER]\tFailed to recieve message from slave: {}", e)     
                // Handle error here  
            }
        }

        match master_to_slave_rx.try_recv() {
            Ok(message) => {
                let encoded = bincode::serialize(&message).expect("Failed to serialize message to slave");
                stream.write(&encoded).unwrap();
                println!("[MASTER]\tSent message to slave: {:#?}", message);
            }
            Err(_) => {
                //eprintln!("[MASTER]\tFailed to read from master_to_slave_rx channel");
            }
        }
    }
}