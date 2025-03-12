use crossbeam_channel as cbc;
use driver_rust::elevio::elev::{HALL_DOWN, HALL_UP, CAB};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::string::String;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;

use crate::config::Config;
use crate::tcp::{self, CallButton, Message};



#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Order {
    call_button: tcp::CallButton,
    in_progress: bool,
}

impl FmtDisplay for Order {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Order: {}, progress: {}",
            self.call_button, self.in_progress
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterQueues {
    pub hall_queue: VecDeque<Order>, 
    pub cab_queues: Vec<VecDeque<Order>>, // Vector of slave queues for internal cab calls. Index corresponds to slave number
}

impl MasterQueues {
    pub fn init() -> MasterQueues {
        let hall_queue: VecDeque<Order> = VecDeque::new(); 
        let cab_queues: Vec<VecDeque<Order>> = Vec::new(); 

        MasterQueues {
            hall_queue,
            cab_queues,
        }
    }

    
    pub fn add_to_hall_queue(&mut self, floor: u8, direction: u8) {
        match direction {
            HALL_UP => {
                self.hall_queue.push_back(Order {
                    call_button: CallButton { floor, call: HALL_UP },
                    in_progress: false,
                });
            }
            HALL_DOWN => {
                self.hall_queue.push_back(Order {
                    call_button: CallButton { floor, call: HALL_DOWN },
                    in_progress: false,
                });
            }
            _ => {
                eprintln!("[MASTER]\tInvalid direction: {}", direction);
                todo!();
            }
        }
    }

    pub fn add_to_cab_queue(&mut self, slave_num: u8, floor: u8) {
        self.cab_queues[slave_num as usize].push_back(Order {
            call_button: CallButton { floor, call: CAB },
            in_progress: false,
        });
    }

    pub fn pop_order(&mut self, order: Order, slave_number: u8) {
        if order.call_button.call == 2 {
            self.cab_queues[slave_number as usize].pop_front();
        } else {
            for i in 0..self.hall_queue.len() {
                if self.hall_queue[i].call_button.floor == order.call_button.floor
                    && self.hall_queue[i].call_button.call == order.call_button.call
                {
                    self.hall_queue.remove(i);
                    break;
                }
            }
        }
    }

    // Simple algorithm for getting the next order. Works for testing purposes, but need to implement more sophisticated version. 
    pub fn get_next_order(&mut self, slave_num: u8) -> Option<Order> {
        //Confirmed working
        if self.cab_queues[slave_num as usize].len() > 0 {
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

            //If all orders are in progress
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
            self.hall_queue, self.cab_queues
        )
    }
}

#[derive(Debug)]
pub struct Master {
    pub config                      : Config,                                                                     
    pub order_queues                : Arc<Mutex<MasterQueues>>,                                          // Vector of slaves order queues
    slave_channels                  : Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>>,   // Vector of slave channels.
    number_of_slaves                : Arc<Mutex<u8>>,                                                    // Variable for number of slaves in operation
    master_to_backup_tx             : Option<cbc::Sender<Message>>,                                      // Channel for sending messages to backup
    backup_disconected_rx           : cbc::Receiver<bool>,                                         // Channel for sending messages to backup
}

impl Master {
    pub fn init
    (
        config: &Config, 
        master_queue: MasterQueues
    ) -> Result<Master, String> 
    {
        let slave_channels: Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>> = Arc::new(Mutex::new(Vec::new()));

        let mut master = Master {
            config                  : config.clone(),
            order_queues            : Arc::new(Mutex::new(master_queue)),
            slave_channels          : slave_channels,
            number_of_slaves        : Arc::new(Mutex::new(0)),
            master_to_backup_tx     : None,
            backup_disconected_rx   : cbc::unbounded().1,
        };

        master.try_connect_to_new_backup();

        let master_port             : u16 = config.master_port;
        let order_queues_clone      : Arc<Mutex<MasterQueues>> = Arc::clone(&master.order_queues);
        let slave_channels_clone    : Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>> = Arc::clone(&master.slave_channels);
        let num_slaves_clone        : Arc<Mutex<u8>> = Arc::clone(&master.number_of_slaves);

        // Thread for listening for new slave connections
        spawn(move || {
            let listener =
                TcpListener::bind("0.0.0.0".to_string() + ":" + master_port.to_string().as_str()).expect("Failed to bind");
            for stream in listener.incoming() {
                let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded();
                let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded();
                let mut locked_channel = slave_channels_clone.lock().unwrap();

                locked_channel.push((master_to_slave_tx, slave_to_master_rx));
                drop(locked_channel);

                let mut locked_num_slaves = num_slaves_clone.lock().unwrap();
                *locked_num_slaves += 1;
                drop(locked_num_slaves);

                order_queues_clone
                    .lock()
                    .unwrap()
                    .cab_queues
                    .push(VecDeque::new());
                println!("[MASTER]\tGot new stream");

                match stream {
                    Ok(stream) => {
                        println!(
                            "[MASTER]\tNew slave connection established: {}",
                            stream.peer_addr().unwrap()
                        );
                        spawn(|| {
                            handle_slave_connection(stream, slave_to_master_tx, master_to_slave_rx)
                        });
                    }
                    Err(e) => {
                        eprintln!("[MASTER]\tFailed to establish connection to slave: {}", e);
                        todo!();
                    }
                }
            }
        });
        Ok(master)
    }

    // Returns a 3 x num_floors matrix for updating panel lights. 
    // 3 x num_floors matrix for [hall up, hall down, cab] lights.
    fn make_light_matrix(&self, slave_number: u8, orders: MasterQueues) -> tcp::Message {
        let mut new_matrix = vec![[false; 3]; self.config.number_of_floors as usize];

        for order in orders.hall_queue.iter() {
            new_matrix[order.call_button.floor as usize][order.call_button.call as usize] = true;
        }

        if orders.cab_queues.len() > 0 {
            orders.cab_queues[slave_number as usize]
                .iter()
                .for_each(|order| {
                    new_matrix[order.call_button.floor as usize][2] = true;
                });
        }
        Message::LightMatrix(new_matrix)
    }

    // Main application loop for master (state machine). Should be refactored to be more readable.
    pub fn master_loop(&mut self) {
        loop {
            if self.backup_disconected_rx.try_recv().is_ok() {
                self.master_to_backup_tx = None;
            }
            if self.master_to_backup_tx.is_none() { //fjerne?
                self.try_connect_to_new_backup();
            }

            let locked_num_slaves = *self.number_of_slaves.lock().unwrap();
            for slave_number in 0..locked_num_slaves {
                let locked_channels = self.slave_channels.lock().unwrap();
                match locked_channels[slave_number as usize].1.try_recv() {
                    Ok(message) => {
                        match message {
                            Message::NewOrder(call_button) => {
                                if call_button.call == CAB  
                                {
                                    let mut orders_locked = self.order_queues.lock().unwrap();
                                    orders_locked.add_to_cab_queue(slave_number, call_button.floor);

                                    println!("[MASTER]\tAdded order to cab queue: {}", call_button);

                                    if self.master_to_backup_tx.is_some() {
                                        match self
                                            .master_to_backup_tx
                                            .as_mut()
                                            .unwrap()
                                            .send(Message::Backup(orders_locked.clone()))
                                        {
                                            Ok(_) => {
                                                let light_matrix = self.make_light_matrix(
                                                    slave_number,
                                                    orders_locked.clone(),
                                                );
                                                locked_channels[slave_number as usize]
                                                    .0
                                                    .send(light_matrix)
                                                    .unwrap();
                                                println!("[MASTER]\tSent light matrix to slave");
                                            }
                                            Err(_) => {
                                                println!(
                                                    "[MASTER]\tFailed to send order to backup"
                                                );
                                                self.master_to_backup_tx = None;
                                            }
                                        }
                                    }
                                } else // Message is a hall call
                                {
                                    let mut orders_locked = self.order_queues.lock().unwrap();
                                    orders_locked
                                        .add_to_hall_queue(call_button.floor, call_button.call);
                                    println!(
                                        "[MASTER]\tAdded order to hall queue: {}",
                                        call_button
                                    );

                                    if self.master_to_backup_tx.is_some() {
                                        match self
                                            .master_to_backup_tx
                                            .as_mut()
                                            .unwrap()
                                            .send(Message::Backup(orders_locked.clone()))
                                        {
                                            Ok(_) => {
                                                // Send lightmatrix to all slaves
                                                for i in 0..locked_num_slaves {
                                                    let light_matrix = self.make_light_matrix(
                                                        i,
                                                        orders_locked.clone(),
                                                    );
                                                    locked_channels[i as usize]
                                                        .0
                                                        .send(light_matrix)
                                                        .unwrap();
                                                    println!(
                                                        "[MASTER]\tSent light matrix to slave {}",
                                                        i
                                                    );
                                                }
                                                println!(
                                                    "[MASTER]\tAdded order to hall queue: {}:{}",
                                                    call_button.floor, call_button.call
                                                );
                                            }
                                            Err(_) => {
                                                println!(
                                                    "[MASTER]\tFailed to send order to backup"
                                                );
                                                self.master_to_backup_tx = None;
                                            }
                                        }
                                    }

                                    // send order list to backup
                                }
                            }

                            // todo: implement order complete for specific order
                            // make function pop index from queue (hall or cab at floor)
                            Message::OrderComplete(call_button) => {
                                let mut orders_locked = self.order_queues.lock().unwrap();

                                orders_locked.pop_order(
                                    Order {
                                        call_button: { call_button },
                                        in_progress: true,
                                    },
                                    slave_number,
                                );

                                if self.master_to_backup_tx.is_some() {
                                    // Send updated order list to backup
                                    match self
                                        .master_to_backup_tx
                                        .as_mut()
                                        .unwrap()
                                        .send(Message::Backup(orders_locked.clone()))
                                    {
                                        Ok(_) => {
                                            for i in 0..locked_num_slaves {
                                                let light_matrix = self
                                                    .make_light_matrix(i, orders_locked.clone());
                                                locked_channels[i as usize]
                                                    .0
                                                    .send(light_matrix)
                                                    .unwrap();
                                                println!(
                                                    "[MASTER]\tSent light matrix to slave {}",
                                                    i
                                                );
                                            }
                                        }
                                        Err(_) => {
                                            println!("[MASTER]\tFailed to send order to backup");
                                            self.master_to_backup_tx = None;
                                            todo!();
                                        }
                                    }
                                }
                            }

                            Message::Idle(state) => { // Variable not used. Do we need it?
                                // Send next order to slave
                                if self.order_queues.lock().unwrap().hall_queue.len() > 0
                                    || self.order_queues.lock().unwrap().cab_queues
                                        [slave_number as usize]
                                        .len()
                                        > 0
                                {
                                    if self.master_to_backup_tx.is_some() {
                                        let mut orders_locked = self.order_queues.lock().unwrap();
                                        let nxt_order = orders_locked.get_next_order(slave_number);
                                        match nxt_order {
                                            Some(order) => {
                                                match self
                                                    .master_to_backup_tx
                                                    .as_mut()
                                                    .unwrap()
                                                    .send(Message::Backup(orders_locked.clone()))
                                                {
                                                    Ok(_) => {
                                                        let message = Message::NewOrder(
                                                            nxt_order.unwrap().call_button,
                                                        );
                                                        locked_channels[slave_number as usize]
                                                            .0
                                                            .send(message)
                                                            .unwrap();
                                                        println!(
                                                            "[MASTER]\t New order message sent"
                                                        );
                                                    }
                                                    Err(_) => {
                                                        println!("[MASTER]\tFailed to send order to backup");
                                                        println!(
                                                            "[MASTER]\tConnecting to a new backup."
                                                        );
                                                        self.master_to_backup_tx = None;
                                                        todo!();
                                                    }
                                                }
                                            }
                                            None => {
                                                //todo!();
                                                println!("[MASTER]\tNo orders available for slave {}", slave_number);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                println!(
                                    "[MASTER]\tReceived unexpected message from slave {:#?}",
                                    message
                                );
                                todo!();
                            }
                        }
                    }
                    Err(_) => {
                        //println!("[MASTER]\tFailed to read from master_to_slave_rx channel");
                        //todo!();
                    }
                }
            }

            //Add a very small sleep to avoid consuming 100% CPU
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    
    fn try_connect_to_new_backup(&mut self) {
        let backup_ip_list: Vec<SocketAddr> = self.config
            .elevator_ip_list
            .iter()
            .map(|ip| format!("{}:{}", ip, self.config.backup_port))
            .map(|addr| addr.parse().expect("Failed to parse IP address"))
            .collect();
    
        for backup_ip in backup_ip_list {
            match TcpStream::connect_timeout(&backup_ip, Duration::from_millis(self.config.tcp_timeout_ms)) {
                Ok(backup_socket) => {
                    // Create channel for backup connection
                    let (master_to_backup_tx, master_to_backup_rx) = cbc::unbounded();
                    let (backup_disconected_tx, backup_disconected_rx) = cbc::unbounded();

                    spawn(|| handle_backup_connection(backup_socket, master_to_backup_rx,backup_disconected_tx));
    
                    println!("[MASTER]\tConnected to backup at {}", backup_ip);
                    self.master_to_backup_tx = Some(master_to_backup_tx);
                    self.backup_disconected_rx=backup_disconected_rx;
                    return;
                }
                Err(e) => {
                    //eprintln!("[MASTER]\tFailed to connect to backup at {}: {}",backup_ip, e);
                    //todo!();
                }
            }
        }
    }
}





// Handles the individual slave connections
fn handle_slave_connection(
    mut stream: TcpStream,
    slave_to_master_tx: cbc::Sender<tcp::Message>,
    master_to_slave_rx: cbc::Receiver<tcp::Message>,
) {
    let mut buffer = [0; 1024];
    stream
        .set_nonblocking(true)
        .expect("Failed to set non-blocking mode on stream");
    loop {

        match stream.read(&mut buffer) {
            Ok(size) => {
                if size > 0 {
                    let recieved: tcp::Message = bincode::deserialize(&buffer[..size])
                        .expect("[MASTER]\tFailed to deserialize message from slave");
                    println!("[MASTER]\tReceived message from slave: {:#?}", recieved);
                    slave_to_master_tx.send(recieved).unwrap();
                }
            }
            Err(e) => {
                //eprintln!("[MASTER]\tFailed to recieve message from slave: {}", e);
                //todo!();
            }
        }

        match master_to_slave_rx.try_recv() {
            Ok(message) => {
                let encoded =
                    bincode::serialize(&message).expect("Failed to serialize message to slave");
                stream.write(&encoded).unwrap();
                println!("[MASTER]\tSent message to slave: {:#?}", message);
            }
            Err(_) => {
                //eprintln!("[MASTER]\tFailed to read from master_to_slave_rx channel");
                //todo!();
            }
        }
    }
}

// Handles the backup connection. 
fn handle_backup_connection(
    mut stream: TcpStream,
    master_to_backup_rx: cbc::Receiver<tcp::Message>,
    backup_disconected_tx: cbc::Sender<bool>,
) {
    loop {
        match master_to_backup_rx.recv() {
            Ok(message) => {
                let encoded =
                    bincode::serialize(&message).expect("Failed to serialize message to backup");
                match stream.write(&encoded){
                    Ok(_)=>{println!("[MASTER]\tSent order to backup: {:#?}", message);}
                    Err(_)=>{
                        eprintln!("[MASTER]\tFailed to send to backup, asuming dead connection");
                        backup_disconected_tx.send(true);//tcp::Message::Error(tcp::ErrorState::Network)).unwrap();
                        return;
                    }
                }
                println!("[MASTER]\tSent order to backup: {:#?}", message);
            }
            Err(_) => {
                eprintln!("[MASTER]\tFailed to read from master_to_slave_rx channel");
                //todo!();
            }
        }
    }
}
