

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
use crate::slave::{self, Direction, ElevatorBehaviour, ElevatorState};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterQueues {
    pub hall_requests       : Vec<(bool, bool)>,   
    pub elevator_states     : Vec<ElevatorState>,
}

impl MasterQueues {
    pub fn init() -> MasterQueues {
        let hall_requests   : Vec<(bool, bool)> = Vec::new(); 
        let elevator_states : Vec<ElevatorState> = Vec::new();

        MasterQueues {
            hall_requests,
            elevator_states,
        }
    }  

    pub fn add_new_elevator(&mut self) {
        self.hall_requests.push((false, false));
        self.elevator_states.push(ElevatorState { 
            behaviour: ElevatorBehaviour::Idle, 
            floor: 0, 
            direction: Direction::Stop, 
            cab_requests: [false; slave::NUMBER_OF_FLOORS as usize]
        });
    }

    pub fn remove_elevator(&mut self, slave_number: u8) {
        self.hall_requests.remove(slave_number as usize);
        self.elevator_states.remove(slave_number as usize);
    }

    pub fn update_elevator_state(&mut self, new_state: ElevatorState, slave_number: u8) {
        self.elevator_states[slave_number as usize] = new_state;
    }


    pub fn get_next_order(&mut self, slave_number: u8) -> tcp::CallButton{
        // run the optimization algorithm
        // return the next order for the slave


    }

    fn to_custom_json(&self) -> String {
        use serde_json::{json, Value, Map};

        // Konverter hall_requests (Vec<(bool, bool)>) til en JSON-array av arrays.
        let hall_requests: Vec<Value> = self.hall_requests
            .iter()
            .map(|&(up, down)| json!([up, down]))
            .collect();

        let mut states = Map::new();
        for (i, state) in self.elevator_states.iter().enumerate() {
            
        }

    }

    pub fn update_hall_requests(&mut self, slave_number: u8, call: tcp::CallButton) {
        match call.call {
            HALL_UP => {
                self.hall_requests[slave_number as usize].0 = true;
            }
            HALL_DOWN => {
                self.hall_requests[slave_number as usize].1 = true;
            }
            _ => {
                println!("[MASTER]\tGot cab call from slave. Exeting");
                return;
            }
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
    backup_disconected_rx           : cbc::Receiver<bool>,                                               // Channel for sending messages to backup
}

impl Master {
    pub fn init
    (
        config: &Config, 
        master_queue: MasterQueues
    ) -> Result<Master, String> 
    {

        let mut master = Master {
            config                  : config.clone(),
            order_queues            : Arc::new(Mutex::new(master_queue)),
            slave_channels          : Arc::new(Mutex::new(Vec::new())),
            slave_elevator_state    : Arc::new(Mutex::new(Vec::new())),
            number_of_slaves        : Arc::new(Mutex::new(0)),
            master_to_backup_tx     : None,
            backup_disconected_rx   : cbc::unbounded().1,
        };

        master.try_connect_to_new_backup();

        let master_port             : u16 = config.master_port;
        let order_queues_clone      : Arc<Mutex<MasterQueues>> = Arc::clone(&master.order_queues);
        let slave_channels_clone    : Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>> = Arc::clone(&master.slave_channels);
        let slave_elev_state_clone  : Arc<Mutex<Vec<ElevatorState>>> = Arc::clone(&master.slave_elevator_state);
        let num_slaves_clone        : Arc<Mutex<u8>> = Arc::clone(&master.number_of_slaves);

        // Thread for listening for new slave connections
        spawn(move || {
            let listener =
                TcpListener::bind("0.0.0.0".to_string() + ":" + master_port.to_string().as_str()).expect("Failed to bind");
            for stream in listener.incoming() {
                let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded();
                let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded();
                let mut locked_channel = slave_channels_clone.lock().unwrap();
                let mut locked_elevator_states = slave_elev_state_clone.lock().unwrap();

                locked_channel.push((master_to_slave_tx, slave_to_master_rx));
                drop(locked_channel);
                locked_elevator_states.push(ElevatorState { behaviour: ElevatorBehaviour::Idle, floor: 0, direction: Direction::Stop });
                drop(locked_elevator_states);

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

    fn optimized_hall_assigner(&mut self) {

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

            let mut locked_num_slaves = *self.number_of_slaves.lock().unwrap();
            for slave_number in 0..locked_num_slaves {
                let mut locked_channels = self.slave_channels.lock().unwrap();
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

                            Message::Idle => {
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
                                            Some(_) => {
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
                            
                            // Recieves an updated state from slave
                            Message::StateUpdate(new_state) => {
                                self.slave_elevator_state.lock().unwrap()[slave_number as usize] = new_state;
                            }

                            // Removes an disconected slave
                            Message::Error(_e) => {
                                locked_channels.remove(slave_number as usize);
                                self.slave_elevator_state.lock().unwrap().remove(slave_number as usize);
                                locked_num_slaves -= 1;
                                
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
                    Err(_) => {}
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
                Err(_e) => { continue; }
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
                match e.kind() {
                    std::io::ErrorKind::WouldBlock => { /* println!("[SLAVE]\t\tNo data available"); */ }
                    _ => {
                        println!("[SLAVE]\t\tFailed to read from stream: {}", e);
                        slave_to_master_tx.send(tcp::Message::Error(tcp::ErrorState::Network)).unwrap();
                    }
                }
            }
        }

        match master_to_slave_rx.try_recv() {
            Ok(message) => {
                let encoded =
                    bincode::serialize(&message).expect("Failed to serialize message to slave");
                match stream.write(&encoded) {
                    Ok(_) => {},
                    Err(_e) => {
                        slave_to_master_tx.send(Message::Error(tcp::ErrorState::Network)).unwrap();
                    }
                }
                println!("[MASTER]\tSent message to slave: {:#?}", message);
            }
            Err(_e) => {
                continue;
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
                        backup_disconected_tx.send(true).unwrap();//tcp::Message::Error(tcp::ErrorState::Network)).unwrap();
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
