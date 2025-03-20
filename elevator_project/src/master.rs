

use crossbeam_channel as cbc;
use driver_rust::elevio::elev::{HALL_DOWN, HALL_UP, CAB};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::string::String;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;
use std::process::Command;
use std::collections::HashMap;

use crate::config::Config;
use crate::tcp::{self, CallButton, Message};
use crate::slave::{self, Direction, ElevatorBehaviour, ElevatorState};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterQueues {
    pub hallRequests       : Vec<[bool; 2]>,   
    pub states     : Vec<ElevatorState>,
}
//endre navn?
impl MasterQueues {
    pub fn init() -> MasterQueues {
        let hallRequests   : Vec<[bool; 2]> = vec![[false; 2]; slave::NUMBER_OF_FLOORS as usize];
        let states : Vec<ElevatorState> = Vec::new();

        MasterQueues {
            hallRequests,
            states,
        }
    }  

    pub fn add_new_elevator(&mut self) {
        self.states.push(ElevatorState { 
            behaviour: ElevatorBehaviour::Idle, 
            floor: 0, 
            direction: Direction::Stop, 
            cab_requests: [false; slave::NUMBER_OF_FLOORS as usize]
        });
    }

    pub fn remove_elevator(&mut self, slave_number: u8) {
        self.states.remove(slave_number as usize);
    }

    pub fn update_elevator_state(&mut self, new_state: ElevatorState, slave_number: u8) {
        self.states[slave_number as usize] = new_state;
    }

    pub fn update_hall_requests(&mut self, call: tcp::CallButton, remove_or_add: bool) { //beire navn, men true for add, false for remove
        match call.call {
            HALL_UP => {
                self.hallRequests[call.floor as usize][0] = remove_or_add;
            }
            HALL_DOWN => {
                self.hallRequests[call.floor as usize][1] = remove_or_add;
            }
            _ => {
                println!("[MASTER]\tGot cab call from slave. Exeting");
                return;
            }
        }
    }

    pub fn get_next_order(&mut self, elevator_number: u8) -> Option<CallButton> {
        // run the optimization algorithm
        // return the next order for the slaves
        let mut orders: HashMap<String, Vec<[bool; 3]>>;

        let input = self.to_custom_json();
        println!("{}", input);

        let output = Command::new("../hall_request_assigner")
            .args(["--includeCab", "--input"])
            .arg(input)
            .output()
            .expect("Failed to start hall_request_assigner");
        
        
        if output.status.success() {
            orders = serde_json::from_slice(&output.stdout).unwrap();
        } 
        else { return None; }

        println!( "[Master] ORDERS: {:#?}", orders);
        

        let elevator= self.states[elevator_number as usize].clone();

        
        match elevator.direction {
            Direction::Down => {
                for i in (0..elevator.floor).rev() {
                    println!("ned {}\n",i);
                    if orders.get(&elevator_number.to_string()).unwrap()[i as usize][HALL_DOWN as usize] {
                        return Some(CallButton { floor: i as u8, call: HALL_DOWN });
                    }
                    if orders.get(&elevator_number.to_string()).unwrap()[i as usize][CAB as usize] {
                        return Some(CallButton { floor: i as u8, call: CAB });
                    }
                }
            }
            Direction::Up => {
                for i in elevator.floor..slave::NUMBER_OF_FLOORS {
                    println!("opp {}\n",i);
                    if orders.get(&elevator_number.to_string()).unwrap()[i as usize][HALL_UP as usize]{
                        return Some(CallButton { floor: i as u8, call: HALL_UP });
                    }
                    if orders.get(&elevator_number.to_string()).unwrap()[i as usize][CAB as usize] {
                        return Some(CallButton { floor: i as u8, call: CAB });
                    }
                }
            }
            Direction::Stop => {
                for i in 0..slave::NUMBER_OF_FLOORS{
                    println!("stop opp{}\n",i);
                    if orders.get(&elevator_number.to_string()).unwrap()[i as usize][HALL_UP as usize]{
                        return Some(CallButton { floor: i as u8, call: HALL_UP });
                    }
                    if orders.get(&elevator_number.to_string()).unwrap()[i as usize][CAB as usize] {
                        return Some(CallButton { floor: i as u8, call: CAB });
                    }
                    if orders.get(&elevator_number.to_string()).unwrap()[i as usize][HALL_DOWN as usize] {
                        return Some(CallButton { floor: i as u8, call: HALL_DOWN });
                    }
                }
            }
        }
        
        return None; 
    }

    
    fn to_custom_json(&self) -> String {
        use serde_json::{json, Value, Map};
        // Konverter hall_requests (Vec<(bool, bool)>) til en JSON-array av arrays.
        let hall_requests: Vec<Value> = self.hallRequests
            .iter()
            .map(|x| json!([x[0], x[1]]))
            .collect();

        let mut states = Map::new();
        for (key, state) in self.states.iter().enumerate() {
            let state_object = json!({
                "floor": state.floor,
                "behaviour": state.behaviour.to_ascii_lowercase(),
                "direction": state.direction.to_ascii_lowercase(),
                "cabRequests": state.cab_requests,
            });
            states.insert(key.to_string(), state_object);
        }

        let result = json!({
            "hallRequests": hall_requests,
            "states": states,
        }); 
        serde_json::to_string(&result).unwrap()
    }
}


impl FmtDisplay for MasterQueues {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Hall queue: {:?}\n\
            Cab queues: {:?}",
            self.hallRequests, self.states
        )
    }
}

#[derive(Debug)]
pub struct Master {
    pub config                      : Config,                                                                     
    pub requests                    : Arc<Mutex<MasterQueues>>,                                          // Vector of slaves order queues
    //orders                          : Vec<[bool; 3]>,                                                    // Vector of slaves order queues
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
            requests                : Arc::new(Mutex::new(master_queue)),
            //orders                  : Vec::new(),
            slave_channels          : Arc::new(Mutex::new(Vec::new())),
            number_of_slaves        : Arc::new(Mutex::new(0)),
            master_to_backup_tx     : None,
            backup_disconected_rx   : cbc::unbounded().1,
        };

        master.try_connect_to_new_backup();

        let master_port             : u16 = config.master_port;
        let slave_channels_clone    : Arc<Mutex<Vec<(cbc::Sender<Message>, cbc::Receiver<Message>)>>> = Arc::clone(&master.slave_channels);
        let num_slaves_clone        : Arc<Mutex<u8>> = Arc::clone(&master.number_of_slaves);
        let requests_clone          : Arc<Mutex<MasterQueues>> = Arc::clone(&master.requests);

        // Thread for listening for new slave connections
        spawn(move || {
            let listener = // e de beire å bruke elevator ip list hær sånn at vi kan huske ordrane til heisa?
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

                let mut locked_requests = requests_clone.lock().unwrap();
                locked_requests.add_new_elevator();
                drop(locked_requests);

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
    fn make_light_matrix(&self, slave_number: u8, requests: MasterQueues) -> tcp::Message {
        let mut new_matrix = vec![[false; 3]; self.config.number_of_floors as usize];

        for (floor, hall_call) in requests.hallRequests.iter().enumerate() {
            match hall_call {
                [false, false] => {
                    new_matrix[floor][0] = false;
                    new_matrix[floor][1] = false;
                }
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

        for (floor, cab_call) in requests.states[slave_number as usize].cab_requests.iter().enumerate() {
            if *cab_call{
                new_matrix[floor][2] = true;
            }
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

            let mut locked_num_slaves = *self.number_of_slaves.lock().unwrap();
            for slave_number in 0..locked_num_slaves {
                let mut locked_channels = self.slave_channels.lock().unwrap();
                match locked_channels[slave_number as usize].1.try_recv() {
                    Ok(message) => {
                        match message {
                            Message::NewOrder(call_button) => {// ditta vil altid vær en hall order no, sant?, so fjerna cab delen
                                let mut requests_locked = self.requests.lock().unwrap();
                                requests_locked.update_hall_requests(call_button, true);
                                println!("[MASTER]\tAdded order to hall queue: {}",call_button);

                                if self.master_to_backup_tx.is_some() { //opdater backup, so alle slava
                                    match self
                                        .master_to_backup_tx
                                        .as_mut()
                                        .unwrap()
                                        .send(Message::Backup(requests_locked.clone()))
                                    {
                                        Ok(_) => {
                                            // Send lightmatrix to all slaves
                                            for i in 0..locked_num_slaves {
                                                let light_matrix = self.make_light_matrix(
                                                    i,
                                                    requests_locked.clone(),
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
                                   // send order list to backup
                                }
                                else {
                                    println!("[MASTER]\tNo backup connected, asuming I am the onely pc in operation");
                                    for i in 0..locked_num_slaves {
                                        let light_matrix = self.make_light_matrix(
                                            i,
                                            requests_locked.clone(),
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
                            }

                            Message::OrderComplete(call_button) => {
                                let mut requests_locked = self.requests.lock().unwrap();

                                requests_locked.update_hall_requests(call_button,false);

                                if self.master_to_backup_tx.is_some() {
                                    // Send updated order list to backup
                                    match self
                                        .master_to_backup_tx
                                        .as_mut()
                                        .unwrap()
                                        .send(Message::Backup(requests_locked.clone()))
                                    {
                                        Ok(_) => {
                                            for i in 0..locked_num_slaves {
                                                let light_matrix = self.make_light_matrix(i, requests_locked.clone());
                                                locked_channels[i as usize].0.send(light_matrix).unwrap();
                                                println!("[MASTER]\tSent light matrix to slave {}",i);
                                            }
                                        }
                                        Err(_) => {
                                            println!("[MASTER]\tFailed to send order to backup");
                                            self.master_to_backup_tx = None;
                                            todo!();
                                        }
                                    }
                                }
                                else{
                                    println!("[MASTER]\tNo backup connected, asuming I am the onely pc in operation");
                                    for i in 0..locked_num_slaves {
                                        let light_matrix = self.make_light_matrix(i, requests_locked.clone());
                                        locked_channels[i as usize].0.send(light_matrix).unwrap();
                                        println!("[MASTER]\tSent light matrix to slave {}",i);
                                    }
                                }
                            }

                            Message::Idle => {
                                // Send next order to slave
                                let mut request_locked = self.requests.lock().unwrap();
                                let nxt_order = request_locked.get_next_order(slave_number);
                                match nxt_order {
                                    Some(_) => {
                                        let message = Message::NewOrder(nxt_order.unwrap());
                                        locked_channels[slave_number as usize]
                                            .0
                                            .send(message)
                                            .unwrap();
                                        println!(
                                            "[MASTER]\t New order message sent"
                                        );
                                    }
                                    None => {
                                        //todo!();
                                        println!("[MASTER]\tNo orders available for slave {}", slave_number);
                                    }
                                }
                            }// idle og state update gjer basacly akkuratt de samme bruke en fungsjon kansje?
                            
                            // Recieves an updated state from slave
                            Message::StateUpdate(new_state) => {
                                let mut request_locked = self.requests.lock().unwrap();
                                request_locked.states[slave_number as usize] = new_state;
                                let light_matrix = self.make_light_matrix(slave_number,request_locked.clone());
                                locked_channels[slave_number as usize].0 .send(light_matrix).unwrap();
                                let nxt_order = request_locked.get_next_order(slave_number);
                                match nxt_order {
                                    Some(_) => {
                                        let message = Message::NewOrder(nxt_order.unwrap());
                                        locked_channels[slave_number as usize]
                                            .0
                                            .send(message)
                                            .unwrap();
                                        println!(
                                            "[MASTER]\t New order message sent"
                                        );
                                    }
                                    None => {
                                        //todo!();
                                        println!("[MASTER]\tNo orders available for slave {}", slave_number);
                                    }
                                }
                            }

                            // Removes an disconected slave
                            Message::Error(_e) => {
                                locked_channels.remove(slave_number as usize);
                                self.requests.lock().unwrap().remove_elevator(slave_number); //kan hende vi ikkje kan fjerne i tilfelle den har mista strøm og kjem tilbake
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
