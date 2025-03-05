#![allow(warnings)]

use core::num;
use std::thread::{spawn, sleep, Builder};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{Incoming, TcpListener, TcpStream, SocketAddr};
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::collections::VecDeque;
use std::string::String;
use std::sync::{Arc, Mutex};
use std::fmt;
use std::time::Duration;
use bincode::config;
use driver_rust::elevio::elev::DIRN_STOP;
use driver_rust::elevio::poll::CallButton;
use serde::{Serialize, Deserialize};
use crossbeam_channel as cbc;

use driver_rust::elevio as e;

use crate::config::Config;
use crate::inputs::{self, listen_for_new_connection};
use crate::slave;
use crate::tcp::{self, Message};


#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Order {
    CallButton: tcp::CallButton,
    in_progress: bool,
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Order: {}, progress: {}", self.CallButton, self.in_progress)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterQueues {
    pub hall_queue: VecDeque<(Order)>,            // (floor, button_type) for external hall calls.
    pub cab_queues: Vec<VecDeque<(Order)>>,       // Vector of slave queues for internal cab calls.  ref driver_rust::elevio::poll::CallButton
}



impl MasterQueues {
    pub fn init() -> MasterQueues {
        let hall_queue      : VecDeque<(Order)>    = VecDeque::new();              // (floor, button_type) for external hall calls.
        let cab_queues      : Vec<VecDeque<(Order)>>     = Vec::new();                   // 
        
        MasterQueues {
            hall_queue,
            cab_queues,
        }
    }

    pub fn add_to_hall_queue(&mut self, floor: u8, direction: u8) {
        match direction 
        {
            0=> {
                // Direction Up
                self.hall_queue.push_back(Order {CallButton: tcp::CallButton { floor, call: 0 }, in_progress: false});
            }
            1 => {
                // Direction Down
                self.hall_queue.push_back(Order {CallButton: tcp::CallButton { floor, call: 1 }, in_progress: false});
            }
            _ => { eprintln!("[MASTER]\tInvalid direction: {}", direction); }
        }
    }

    pub fn add_to_cab_queue(&mut self, slave_num: u8, floor: u8) {
        self.cab_queues[slave_num as usize].push_back(Order {CallButton: tcp::CallButton { floor, call: 2 }, in_progress: false});
    }

    pub fn get_next_order(&mut self, slave_num: u8) -> Option<Order> {
        //confirmed working
        if self.cab_queues[slave_num as usize].len() > 0 
        {
            let mut order = *self.cab_queues[slave_num as usize].front().unwrap(); 
            order.in_progress = true;
            return Some(order);
        }

        // Need work
        else {    
            for i in 0..self.hall_queue.len() {
                if self.hall_queue[i].in_progress == false {
                    self.hall_queue[i].in_progress = true;
                    return Some(self.hall_queue[i]);
                }      
            }   

            //den kjem hit vist alle orders er i progress 
            return None;        
  
        }
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
pub struct Master 
{
    pub config              : Config,                                                   // Config struct                                 
    slaves_ip               : Vec<String>,                                              // Vector of slaves IP addresses
    pub order_queues        : Arc<Mutex<MasterQueues>>,                                             // Vector of slaves order queues
    //incoming_clients_rx     : cbc::Receiver<TcpStream>,                                 // Incoming connections
    slave_channels          : Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>>,      // Vector of slave channels. Sygt som fy
    num_slaves              : Arc<Mutex<u8>>,                                                           // Variable for number of slaves in operation
    master_to_backup_tx     : Option<cbc::Sender<Message>>
}

impl Master 
{
    pub fn init(
        config              : &Config,
        master_ip           : &String,
        master_queue        : MasterQueues,
    ) -> Result<Master, String> 
    {

        let conf            : Config    = config.clone();
                                                                                                
        // Thread for listening for new slave connections
        //let (incoming_conn_tx, incoming_conn_rx) = cbc::unbounded();
        let mut slave_channels : Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>> = Arc::new(Mutex::new(Vec::new()));

        let mut master = Master {
            config                  : config.clone(),                           
            slaves_ip               : config.elevator_ip_list.clone(),                         
            order_queues            : Arc::new(Mutex::new(master_queue)),                   
            //incoming_clients_rx     : incoming_conn_rx,                       
            slave_channels          : slave_channels,        
            num_slaves              : Arc::new(Mutex::new(0)),          
            master_to_backup_tx     : connect_to_new_backup(config.clone()),     
        }; 

        // find an avaliable backup to connect to
        
    
        let master_port = config.master_port;
        let order_queues_clone: Arc<Mutex<MasterQueues>> = Arc::clone(&master.order_queues);
        let slave_channels_clone: Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>> = Arc::clone(&master.slave_channels);
        let num_slaves_clone: Arc<Mutex<u8>> = Arc::clone(&master.num_slaves);

        println!("[MASTER]\tListening for slaves on port {}", master_port);
        spawn(move || {
            let listener  = TcpListener::bind("0.0.0.0".to_string() + ":" + master_port.to_string().as_str()).expect("Failed to bind");
            for stream in listener.incoming() {
                let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded();
                let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded();
                let mut locked_channel = slave_channels_clone.lock().unwrap();
                
                locked_channel.push((master_to_slave_tx, slave_to_master_rx));
                drop(locked_channel);
                
                let mut locked_num_slaves = num_slaves_clone.lock().unwrap();
                *locked_num_slaves += 1;
                drop(locked_num_slaves);

                order_queues_clone.lock().unwrap().cab_queues.push(VecDeque::new());
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

        Ok(master)
    
    }




    fn make_light_matrix(&self, slave_number: u8, orders: MasterQueues) -> tcp::Message {
        // 3 x num_floors matrix for [hall up, hall down, cab] lights
        let mut new_matrix = vec![[false; 3]; self.config.number_of_floors as usize];
    
        for order in orders.hall_queue.iter() {
            new_matrix[order.CallButton.floor as usize][order.CallButton.call as usize] = true;
        }
    
        if orders.cab_queues.len() > 0
        {
            orders.cab_queues[slave_number as usize].iter().for_each(|order| {
                new_matrix[order.CallButton.floor as usize][2] = true;
            });
        }
        Message::LightMatrix(new_matrix)
    }


    pub fn master_loop(&mut self) {
        loop {
            let mut locked_num_slaves = *self.num_slaves.lock().unwrap();
            for slave_number in 0..locked_num_slaves {
                let mut locked_channels = self.slave_channels.lock().unwrap();
                match locked_channels[slave_number as usize].1.try_recv() {
                    Ok(message) => 
                    {                
                        match message {
                            Message::NewOrder(call_button) => {
                                if call_button.call == 2     // Cab call
                                {
                                    let mut orders_locked = self.order_queues.lock().unwrap();
                                    orders_locked.add_to_cab_queue(slave_number, call_button.floor);                                   
                  
                                    println!("[MASTER]\tAdded order to cab queue: {}", call_button );

                                    match self.master_to_backup_tx.as_mut().unwrap().send(Message::Backup(orders_locked.clone())) {
                                        Ok(_) => {
                                            println!("test  ");
                                            let light_matrix = self.make_light_matrix(slave_number, orders_locked.clone());
                                            println!("test 2");
                                            locked_channels[slave_number as usize].0.send(light_matrix).unwrap();
                                            println!("[MASTER]\tSent light matrix to slave");
                                        }
                                        Err(_) => {    
                                            println!("[MASTER]\tFailed to send order to backup");
                                            println!("[MASTER]\tConnecting to a new backup.");
                                            self.master_to_backup_tx = connect_to_new_backup(self.config.clone());
                                        }
                                    }                                                
                                } 
                                else // Is hall call
                                {
                                    // Add order to hall queue
                                    let mut orders_locked = self.order_queues.lock().unwrap();
                                    orders_locked.add_to_hall_queue(call_button.floor, call_button.call);
                                    println!("[MSATER]\tAdded order to hall queue: {}", call_button);
                                    
                                    // send order list to backup
                                    match self.master_to_backup_tx.as_mut().unwrap().send(Message::Backup(orders_locked.clone())) {
                                        Ok(_) => {
                                            // send lightmatrix to all slaves
                                            for i in 0..locked_num_slaves {
                                                
                                                let light_matrix = self.make_light_matrix( i, orders_locked.clone());
                                                locked_channels[i as usize].0.send(light_matrix).unwrap();
                                                println!("[MASTER]\tSent light matrix to slave {}", i);
                                            }
                                            println!("[MASTER]\tAdded order to hall queue: {}:{}", call_button.floor, call_button.call);
                                        }
                                        Err(_) => {
                                            println!("[MASTER]\tFailed to send order to backup");
                                            println!("[MASTER]\tConnecting to a new backup.");
                                            self.master_to_backup_tx = connect_to_new_backup(self.config.clone());
                                        }
                                    }
                                }
                            }

                            // todo: implement order complete for specific order
                            // make function pop index from queue (hall or cab at floor)

                            Message::OrderComplete(call_button) => {
                                println!("[MASTER]\tRecieved OrderComplete: {}", call_button);
                                // Remove order from Cab queue
                                if call_button.call == 2 {
                                    let mut orders_locked = self.order_queues.lock().unwrap();
                                    orders_locked.cab_queues[slave_number as usize].pop_front();
                                    print!("[MASTER]\tRemoved order from cab queue");
                        

                                    // Send updated order list to backup                                
                                    match self.master_to_backup_tx.as_mut().unwrap().send(Message::Backup(orders_locked.clone())) {
                                        Ok(_) => {
                                            let light_matrix = self.make_light_matrix(slave_number, orders_locked.clone());
                                            locked_channels[slave_number as usize].0.send(light_matrix).unwrap();
                                            println!("[MASTER]\tSent light matrix to slave {}", slave_number);                                                                        
                                        }
                                        Err(_) => {
                                            println!("[MASTER]\tFailed to send order to backup");
                                            println!("[MASTER]\tConnecting to a new backup.");
                                            self.master_to_backup_tx = connect_to_new_backup(self.config.clone());
                                        }
                                    }
                                } 
                                else {    // Remove order from Hall queue
                                    let mut orders_locked = self.order_queues.lock().unwrap();
                                    orders_locked.hall_queue.pop_front();
                                    print!("[MASTER]\tRemoved order from hall queue");
                            

                                    // Send updated order list to backup                                
                                    match self.master_to_backup_tx.as_mut().unwrap().send(Message::Backup(orders_locked.clone())) {
                                        Ok(_) => {
                                            for i in 0..locked_num_slaves{
                                                let light_matrix = self.make_light_matrix(i, orders_locked.clone());
                                                locked_channels[i as usize].0.send(light_matrix).unwrap();
                                                println!("[MASTER]\tSent light matrix to slave {}", i);   
                                            }                                                                     
                                        }
                                        Err(_) => {
                                            println!("[MASTER]\tFailed to send order to backup");
                                            println!("[MASTER]\tConnecting to a new backup.");
                                            self.master_to_backup_tx = connect_to_new_backup(self.config.clone());
                                        }
                                    }
                                }
                            }
        
                            Message::Idle(state) => {
                                // Send next order to slave
                                if  self.order_queues.lock().unwrap().hall_queue.len() > 0 || 
                                    self.order_queues.lock().unwrap().cab_queues[slave_number as usize].len() > 0
                                {
                                    let mut orders_locked = self.order_queues.lock().unwrap();
                                    let nxt_order = orders_locked.get_next_order(slave_number);
                                    match nxt_order {
                                        Some(order) => {
                                            match self.master_to_backup_tx.as_mut().unwrap().send(Message::Backup(orders_locked.clone())) {
                                                Ok(_) => {
                                                    let message = Message::NewOrder(nxt_order.unwrap().CallButton); 
                                                    locked_channels[slave_number as usize].0.send(message).unwrap();
                                                    println!("[MASTER]\t New order message sent");
                                                }
                                                Err(_) => {
                                                    println!("[MASTER]\tFailed to send order to backup");
                                                    println!("[MASTER]\tConnecting to a new backup.");
                                                    self.master_to_backup_tx = connect_to_new_backup(self.config.clone());
                                                }
                                            }
                                        }
                                        None => {
                                            // Eventuellt
                                        }
                                    }
                                }
                            }
        
                            _ => {
                                println!("[MASTER]\tReceived unexpected message from slave {:#?}", message);
                            }
                            
                        }
                    }
                    Err(_) => {
                        //println!("[MASTER]\tFailed to read from master_to_slave_rx channel");
                    }
                }   
            }

            // Optional: Add a very small sleep to avoid consuming 100% CPU
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}



fn connect_to_new_backup(config: Config) -> Option<cbc::Sender<tcp::Message>>
{
    let backup_ip_list: Vec<SocketAddr> = config.elevator_ip_list.iter()
    .map(|ip| format!("{}:{}", ip, config.backup_port))
    .map(|addr| addr.parse().expect("Failed to parse IP address"))
    .collect();

    for backup_ip in backup_ip_list {
        match TcpStream::connect_timeout(&backup_ip, Duration::from_millis(config.tcp_timeout_ms)) {
            Ok(backup_socket) => {
                // Create channel for backup connection
                let (master_to_backup_tx, master_to_backup_rx) = cbc::unbounded();
                spawn(|| handle_backup_connection(backup_socket, master_to_backup_rx));

                println!("[MASTER]\tConnected to backup at {}", backup_ip);
                return Some(master_to_backup_tx);
            }
            Err(e) => {
                eprintln!("[MASTER]\tFailed to connect to backup at {}: {}", backup_ip, e);
            }
        }
    }
    None
}


// Implementer funksjon for å håndtere meldinger fra slave
fn handle_slave_connection(mut stream: TcpStream, slave_to_master_tx: cbc::Sender<tcp::Message>, master_to_slave_rx: cbc::Receiver<tcp::Message>) {
    let mut buffer = [0; 1024];
    loop {
        stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
    
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size > 0 {
                    let recieved: tcp::Message = bincode::deserialize(&buffer[..size]).expect("[MASTER]\tFailed to deserialize message from slave");
                    println!("[MASTER]\tReceived message from slave: {:#?}", recieved);
                    slave_to_master_tx.send(recieved).unwrap();
                }
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


fn handle_backup_connection
(
    mut stream: TcpStream,
    master_to_backup_rx: cbc::Receiver<tcp::Message>,
)
{
    let mut buffer = [0; 1024];
    loop {
        //stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");

        match master_to_backup_rx.recv() {
            Ok(message) => {
                let encoded = bincode::serialize(&message).expect("Failed to serialize message to backup");
                stream.write(&encoded).unwrap();
                println!("[MASTER]\tSent order to backup: {:#?}", message);
            }
            Err(_) => {
                //eprintln!("[MASTER]\tFailed to read from master_to_slave_rx channel");
            }
        }
    }
}


