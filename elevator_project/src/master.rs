use crossbeam_channel as cbc;
use driver_rust::elevio::elev::{CAB, HALL_DOWN, HALL_UP};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use bincode;
use debug_print::debug_println as dprintln;
use std::collections::HashMap;
use std::fmt::{Display as FmtDisplay, Formatter as FmtFormatter, Result as FmtResult};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::string::String;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;
use std::result::Result;
use std::io::Error;

use crate::config::{Config, BUFFER_SIZE, NUMBER_OF_ELEVATORS, NUMBER_OF_FLOORS};
use crate::slave::{Direction, ElevatorBehaviour, ElevatorState};
use crate::io_datastructures::{self, CallButton, Message};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderRequests {
    pub hall_requests: [[bool; 2]; NUMBER_OF_FLOORS],
    pub states: [ElevatorState; NUMBER_OF_ELEVATORS],
}

impl FmtDisplay for OrderRequests {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f,
            "Hall queue: {:?}\n\
            Cab queues: {:?}",
            self.hall_requests, self.states
        )
    }
}

impl OrderRequests {
    pub fn init() -> OrderRequests {
        let hall_requests: [[bool; 2]; NUMBER_OF_FLOORS] = [[false; 2]; NUMBER_OF_FLOORS];
        let states: [ElevatorState; NUMBER_OF_ELEVATORS] = [ElevatorState::init(); NUMBER_OF_ELEVATORS];

        OrderRequests {
            hall_requests,
            states,
        }
    }

    pub fn update_hall_requests(&mut self, call: io_datastructures::CallButton, add_or_remove: bool) {
        // true for add, false for remove
        match call.call {
            HALL_UP => {
                self.hall_requests[call.floor as usize][0] = add_or_remove;
            }
            HALL_DOWN => {
                self.hall_requests[call.floor as usize][1] = add_or_remove;
            }
            _ => {
                dprintln!("[MASTER]\tGot cab call from slave. Exiting");
            }
        }
    }

    /// Run the optimization algorithm an return the next order for the slave.
    pub fn get_next_order(&mut self, slave_number: usize) -> Result<Option<CallButton>, Error> {
        let hall_requests: Vec<Value> = self
            .hall_requests
            .iter()
            .map(|x| json!([x[0], x[1]]))
            .collect();

        let mut states = Map::new();
        for (key, state) in self.states.iter().enumerate() {
            if state.behaviour != ElevatorBehaviour::OutOfOrder {
                let state_object = json!({
                "floor": state.floor,
                "behaviour": state.behaviour.to_ascii_lowercase(),
                "direction": state.direction.to_ascii_lowercase(),
                "cabRequests": state.cab_requests,
                });
                states.insert(key.to_string(), state_object);
            }
        }

        let result = json!({
            "hallRequests": hall_requests,
            "states": states,
        });


        let input = serde_json::to_string(&result)?;

        let output = Command::new("../hall_request_assigner")
            .args(["--includeCab", "--input"])
            .arg(input)
            .output()?;

            let orders: HashMap<String, Vec<[bool; 3]>> = if output.status.success() {
                serde_json::from_slice(&output.stdout)?
            } else {
                return Ok(None);
            };

        
            // Prøvde å skrive om denne delen for mindre repetetiv kode + error-handling. Logikk må verifiseres
            let elevator = self.states[slave_number];
            if elevator.behaviour != ElevatorBehaviour::OutOfOrder {
                let elevator_orders = match orders.get(&slave_number.to_string()) {
                    Some(orders) => orders,
                    None => {
                        dprintln!("[MASTER]\tNo orders found for slave {}", slave_number);
                        return Ok(None);
                    }
                };
            
                // Helper function to check and create call button
                let check_button = |floor: u8, call_type: u8| -> Option<CallButton> {
                    if floor < NUMBER_OF_FLOORS as u8 && 
                       call_type < 3 && 
                       elevator_orders[floor as usize][call_type as usize] {
                        Some(CallButton { floor, call: call_type })
                    } else {
                        None
                    }
                };
            
                match elevator.direction {
                    Direction::Down => {
                        for i in (0..elevator.floor).rev() {
                            // First check hall down buttons (same direction)
                            if let Some(button) = check_button(i, HALL_DOWN) {
                                return Ok(Some(button));
                            }
                            // Then check cab buttons
                            if let Some(button) = check_button(i, CAB) {
                                return Ok(Some(button));
                            }
                        }
                    }
                    Direction::Up => {
                        // Check floors above current position
                        for i in elevator.floor..NUMBER_OF_FLOORS as u8 {
                            // First check hall up buttons (same direction)
                            if let Some(button) = check_button(i, HALL_UP) {
                                return Ok(Some(button));
                            }
                            // Then check cab buttons
                            if let Some(button) = check_button(i, CAB) {
                                return Ok(Some(button));
                            }
                        }
                    }
                    Direction::Stop => {
                        
                        //First do cab calls
                        for i in elevator.floor..NUMBER_OF_FLOORS as u8  {
                            if let Some(button) = check_button(i,CAB) {
                                return Ok(Some(button));
                            }
                        }
                        for i in (0..elevator.floor).rev()   {
                            if let Some(button) = check_button(i,CAB) {
                                return Ok(Some(button));
                            }
                        }
                        // Then check floors above current position
                        for i in elevator.floor..NUMBER_OF_FLOORS as u8 {
                            // Check all button types
                            for call_type in [HALL_UP, HALL_DOWN] {
                                if let Some(button) = check_button(i, call_type) {
                                    return Ok(Some(button));
                                }
                            }
                        }
                        
                        // Then check floors below current position
                        for i in (0..elevator.floor).rev() {
                            // Check all button types
                            for call_type in [HALL_UP, HALL_DOWN, CAB] {
                                if let Some(button) = check_button(i, call_type) {
                                    return Ok(Some(button));
                                }
                            }
                        }
                    }
                }
            }
            Ok(None)
        }
    

    pub fn to_custom_json(&self) -> String {
        match serde_json::to_string(&self) {
            Ok(json) => json,
            Err(e) => {
                dprintln!("[MASTER]\tFailed to serialize OrderRequests to JSON: {}", e);
                String::new()
            }
        }
    }
}



#[derive(Debug)]
pub struct Master {
    pub config: Config,
    pub requests: Arc<Mutex<OrderRequests>>, // Vector of slaves order queues
    slave_channels:
        Arc<Mutex<[Option<(cbc::Sender<Message>, cbc::Receiver<Message>)>; NUMBER_OF_ELEVATORS]>>, // Vector of slave channels.
    master_to_backup_tx: Option<cbc::Sender<Message>>, // Channel for sending messages to backup
    backup_disconected_rx: cbc::Receiver<bool>,        // Channel for sending messages to backup
}

impl Master {
    pub fn init(config: &Config, order_requests: OrderRequests) -> Result<Master, String> {
        let mut master = Master {
            config: config.clone(),
            requests: Arc::new(Mutex::new(order_requests)),
            slave_channels: Arc::new(Mutex::new([const { None }; NUMBER_OF_ELEVATORS])),
            master_to_backup_tx: None,
            backup_disconected_rx: cbc::unbounded().1,
        };

        master.try_connect_to_new_backup();

        let master_port: u16 = config.master_port;
        let ip_config_clone: [Ipv4Addr; NUMBER_OF_ELEVATORS] = config.elevator_ip_list;
        let slave_channels_clone: Arc<
            Mutex<[Option<(cbc::Sender<Message>, cbc::Receiver<Message>)>; NUMBER_OF_ELEVATORS]>,
        > = Arc::clone(&master.slave_channels);
        let requests_clone: Arc<Mutex<OrderRequests>> = Arc::clone(&master.requests);

        // Thread for listening for new slave connections
        spawn(move || {
            let listener = // e de beire å bruke elevator ip list hær sånn at vi kan huske ordrane til heisa?
            TcpListener::bind("0.0.0.0".to_string() + ":" + master_port.to_string().as_str()).expect("Failed to bind");
            for stream in listener.incoming() {
                let slave_number: usize;
                match stream.as_ref().unwrap().peer_addr().unwrap().ip() {
                    std::net::IpAddr::V4(ip) => {
                        let ip = ip_config_clone.iter().position(|&x| x == ip).unwrap();
                        slave_number = ip;
                    }
                    std::net::IpAddr::V6(_ip) => {
                        panic!("Fant IP_V6 adresse")
                    } // Panic for invalid ip
                };

                let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded();
                let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded();

                let mut locked_channel = slave_channels_clone.lock().unwrap();
                locked_channel[slave_number] = Some((master_to_slave_tx, slave_to_master_rx));

                let locked_requests = requests_clone.lock().unwrap();

                dprintln!("[MASTER]\tGot new stream: {}", slave_number);

                match stream {
                    Ok(stream) => {
                        dprintln!(
                            "[MASTER]\tNew slave connection established: {}",
                            stream.peer_addr().unwrap()
                        );
                        spawn(move || {
                            handle_slave_connection(stream, slave_to_master_tx, master_to_slave_rx)
                        });

                        // send previous cab orders to slave
                        dprintln!("[MASTER]\tSending previous orders to slave");
                        locked_channel[slave_number]
                            .as_ref()
                            .unwrap()
                            .0
                            .send(Message::StateUpdate(locked_requests.states[slave_number]))
                            .unwrap();
                        drop(locked_requests);
                        drop(locked_channel);
                    }
                    Err(_) => {
                        dprintln!("[MASTER]\tFailed to establish connection to slave");
                    }
                }
            }
        });
        Ok(master)
    }

    // Returns a 2 x num_floors matrix for updating panel lights.
    // 2 x num_floors matrix for [hall up, hall down] lights.
    fn make_light_matrix(&self, requests: OrderRequests) -> io_datastructures::Message {
        let mut new_matrix = [[false; 2]; NUMBER_OF_FLOORS];

        for (floor, hall_call) in requests.hall_requests.iter().enumerate() {
            match hall_call {
                [false, false] => {}
                [true, false] => {
                    new_matrix[floor][0] = true;
                }
                [false, true] => {
                    new_matrix[floor][1] = true;
                }
                [true, true] => {
                    new_matrix[floor][0] = true;
                    new_matrix[floor][1] = true;
                }
            }
        }

        Message::LightMatrix(new_matrix)
    }

    fn update_backup(&self, requests: OrderRequests) -> Result<(), io_datastructures::ErrorState> {
        if self.master_to_backup_tx.is_some() {
            match self
                .master_to_backup_tx
                .as_ref()
                .unwrap()
                .send(Message::Backup(requests))
            {
                Ok(_) => {
                    dprintln!("[MASTER]\tSent order to backup");
                    return Ok(());
                }
                Err(_) => {
                    dprintln!("[MASTER]\tFailed to send order to backup");
                    return Err(io_datastructures::ErrorState::Network);
                }
            }
        }
        dprintln!("[MASTER]\tNo backup connected, asuming I am the onely pc in operation");
        Err(io_datastructures::ErrorState::Network)
    }

    // Main application loop for master (state machine). Should be refactored to be more readable.
    pub fn master_loop(&mut self) {
        loop {
            if self.backup_disconected_rx.try_recv().is_ok() {
                self.master_to_backup_tx = None;
            }
            if self.master_to_backup_tx.is_none() {
                self.try_connect_to_new_backup();
            }

            for slave_number in 0..NUMBER_OF_ELEVATORS {
                let mut locked_channels = self.slave_channels.lock().unwrap();

                // No connected slave at this IP adress. Skip to next
                if locked_channels[slave_number].is_none() {
                    continue;
                }

                // if the slave is connected, check for messages:
                match locked_channels[slave_number].clone().unwrap().1.try_recv() {
                    Ok(message) => {
                        match message {
                            Message::NewOrder(call_button) => {
                                let mut locked_requests = self.requests.lock().unwrap();
                                locked_requests.update_hall_requests(call_button, true);

                                dprintln!("[MASTER]\tAdded order to hall queue: {}", call_button);

                                match self.update_backup(locked_requests.clone()) {
                                    Ok(_) => {}
                                    Err(_) => self.master_to_backup_tx = None,
                                }

                                for i in 0..NUMBER_OF_ELEVATORS {
                                    if locked_channels[i].is_none() {
                                        continue;
                                    }
                                    let light_matrix =
                                        self.make_light_matrix(locked_requests.clone());
                                    locked_channels[i]
                                        .clone()
                                        .unwrap()
                                        .0
                                        .send(light_matrix)
                                        .unwrap();
                                    dprintln!("[MASTER]\tSent light matrix to slave {}", i);
                                }
                            }

                            Message::OrderComplete(call_button) => {
                                let mut locked_requests = self.requests.lock().unwrap();
                                locked_requests.update_hall_requests(call_button, false);

                                match self.update_backup(locked_requests.clone()) {
                                    Ok(_) => {}
                                    Err(_) => self.master_to_backup_tx = None,
                                }

                                for i in 0..NUMBER_OF_ELEVATORS {
                                    if locked_channels[i].is_some() {
                                        let light_matrix =
                                            self.make_light_matrix(locked_requests.clone());
                                        locked_channels[i]
                                            .clone()
                                            .unwrap()
                                            .0
                                            .send(light_matrix)
                                            .unwrap();
                                        dprintln!("[MASTER]\tSent light matrix to slave {}", i);
                                    }
                                }
                            }

                            // Recieves an updated state from slave
                            Message::StateUpdate(new_state) => {
                                let mut locked_requests = self.requests.lock().unwrap();
                                locked_requests.states[slave_number] = new_state;

                                match self.update_backup(locked_requests.clone()) {
                                    Ok(_) => {}
                                    Err(_) => self.master_to_backup_tx = None,
                                }
                                dprintln!("[MASTER]\tNew state update from slave:\t{}", new_state);

                              
                                match locked_requests.get_next_order(slave_number) {
                                    Ok(Some(nxt_order)) => {
                                        let message = Message::NewOrder(nxt_order);
                                        locked_channels[slave_number]
                                            .clone()
                                            .unwrap()
                                            .0
                                            .send(message)
                                            .unwrap();
                                        dprintln!(
                                            "[MASTER]\tNew order message sent to slave:{}, order{}",
                                            slave_number,
                                            nxt_order
                                        );
                                    }
                                    Ok(None) => {
                                        dprintln!(
                                            "[MASTER]\tNo orders available for slave:\t{}",
                                            slave_number
                                        );
                                    }

                                    Err(e) => {
                                        dprintln!(
                                            "[MASTER]\tFailed to get next order for slave: {}",
                                            e
                                        );
                                    }
                                }
                            }

                            // Removes an disconected slave
                            Message::Error(e) => match e {
                                io_datastructures::ErrorState::Network => {
                                    self.requests.lock().unwrap().states[slave_number].behaviour =
                                        ElevatorBehaviour::OutOfOrder;

                                    match self.update_backup(self.requests.lock().unwrap().clone())
                                    {
                                        Ok(_) => {}
                                        Err(_) => self.master_to_backup_tx = None,
                                    }

                                    dprintln!("[MASTER]\tSlave {} disconnected", slave_number);
                                    locked_channels[slave_number] = None;
                                }
                                io_datastructures::ErrorState::EmergancyStop => {
                                    dprintln!(
                                        "[MASTER]\tSlave {} has emergancy stop",
                                        slave_number
                                    );
                                    self.requests.lock().unwrap().states[slave_number].behaviour =
                                        ElevatorBehaviour::OutOfOrder;

                                    match self.update_backup(self.requests.lock().unwrap().clone())
                                    {
                                        Ok(_) => {}
                                        Err(_) => self.master_to_backup_tx = None,
                                    }
                                }
                            },
                            _ => {
                                dprintln!(
                                    "[MASTER]\tReceived unexpected message from slave {:#?}",
                                    message
                                );
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
        let backup_ip_list: Vec<SocketAddr> = self
            .config
            .elevator_ip_list
            .iter()
            .map(|ip| format!("{}:{}", ip, self.config.backup_port))
            .map(|addr| addr.parse().expect("Failed to parse IP address"))
            .collect();

        for backup_ip in backup_ip_list {
            match TcpStream::connect_timeout(
                &backup_ip,
                Duration::from_millis(self.config.tcp_timeout_ms),
            ) {
                Ok(backup_socket) => {
                    // Create channel for backup connection
                    let (master_to_backup_tx, master_to_backup_rx) = cbc::unbounded();
                    let (backup_disconected_tx, backup_disconected_rx) = cbc::unbounded();

                    spawn(|| {
                        handle_backup_connection(
                            backup_socket,
                            master_to_backup_rx,
                            backup_disconected_tx,
                        )
                    });

                    dprintln!("[MASTER]\tConnected to backup at {}", backup_ip);
                    self.master_to_backup_tx = Some(master_to_backup_tx);
                    self.backup_disconected_rx = backup_disconected_rx;
                    return;
                }
                Err(_e) => {
                    continue;
                }
            }
        }
    }
}

// Handles the individual slave connections
fn handle_slave_connection(
    mut stream: TcpStream,
    slave_to_master_tx: cbc::Sender<io_datastructures::Message>,
    master_to_slave_rx: cbc::Receiver<io_datastructures::Message>,
) {
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

    // TTL is set to 3 to avoid packages being forwarded to other networks
    stream.set_ttl(3).expect("Failed to set TTL on stream");
    stream
        .set_nodelay(true)
        .expect("Failed to set nodelay on stream");
    stream
        .set_nonblocking(true)
        .expect("Failed to set non-blocking mode on stream");

    loop {
        match stream.read(&mut buffer) {
            Ok(size) => {
                if size > 0 {
                    let msg: Message = bincode::deserialize::<Message>(&buffer[..size])
                        .expect("[MASTER]\tFailed to deserialize message from slave");
                    slave_to_master_tx.send(msg).unwrap();
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {}
                _ => {
                    dprintln!("[SLAVE]\t\tFailed to read from stream: {}", e);
                    slave_to_master_tx
                        .send(io_datastructures::Message::Error(io_datastructures::ErrorState::Network))
                        .unwrap();
                }
            },
        }

        match master_to_slave_rx.try_recv() {
            Ok(message) => {
                let encoded: Vec<u8> =
                    bincode::serialize(&message).expect("Failed to serialize message to slave");
                match stream.write_all(&encoded) {
                    Ok(_) => {}
                    Err(_e) => {
                        slave_to_master_tx
                            .send(Message::Error(io_datastructures::ErrorState::Network))
                            .unwrap();
                    }
                }
            }
            Err(_e) => {
                continue;
            }
        }
    }
}











fn handle_backup_connection(
    mut stream: TcpStream,
    master_to_backup_rx: cbc::Receiver<io_datastructures::Message>,
    backup_disconected_tx: cbc::Sender<bool>,
) {
    // TTL is set to 3 to avoid packages being forwarded to other networks
    stream.set_ttl(3).expect("Failed to set TTL on stream");
    stream
        .set_nodelay(true)
        .expect("Failed to set nodelay on stream");
    stream
        .set_nonblocking(true)
        .expect("Failed to set non-blocking mode on stream");

    loop {
        match master_to_backup_rx.recv() {
            Ok(message) => {
                let encoded: Vec<u8> =
                    bincode::serialize(&message).expect("Failed to serialize message to backup");
                match stream.write(&encoded) {
                    Ok(_) => {
                        dprintln!("[MASTER]\tSent order to backup: {:#?}", message);
                    }
                    Err(_) => {
                        dprintln!("[MASTER]\tFailed to send to backup, asuming dead connection");
                        backup_disconected_tx.send(true).unwrap();
                        return;
                    }
                }
                dprintln!("[MASTER]\tSent order to backup: {:#?}", message);
            }
            Err(_) => {
                dprintln!("[MASTER]\tFailed to read from master_to_slave_rx channel");
            }
        }
    }
}
