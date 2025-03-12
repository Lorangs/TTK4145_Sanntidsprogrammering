#![allow(warnings)]

use core::num;
use std::thread::{spawn, sleep, Builder};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{Incoming, TcpListener, TcpStream};
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::collections::VecDeque;
use std::string::String;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fmt, result};
use bincode::config;
use driver_rust::elevio::elev::DIRN_STOP;
use driver_rust::elevio::poll::CallButton;
use serde::{Serialize, Deserialize};
use crossbeam_channel::{self as cbc, TryReadyError, TryRecvError};

use driver_rust::elevio as e;

use crate::config::Config;
use crate::inputs::{self};
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
        let hall_queue      : VecDeque<(Order)>          = VecDeque::new();              // (floor, button_type) for external hall calls.
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

    pub fn get_next_order(&mut self, slave_num: u8) -> Order {
        if self.cab_queues[slave_num as usize].len() > 0 
        {
            let mut order = *self.cab_queues[slave_num as usize].front().unwrap();
            order.in_progress = true;
            return order;
        }

        else {    
            for i in 0..self.hall_queue.len() {
                if self.hall_queue[i].in_progress == false {
                    self.hall_queue[i].in_progress = true;
                    return self.hall_queue[i];
                }      
            }   
            //den kjem hit vist alle orders er i progress 
            return Order {CallButton: tcp::CallButton { floor: 0, call: 0 }, in_progress: false};        
  
        }
    }
}


#[derive(Debug, Clone)]
pub struct Master{
    pub config              : Config, 
    master_to_backup_tx     : cbc::Sender<Message>,
    backup_to_master_rx     : cbc::Receiver<Message>,

}

impl Master {
    pub fn init(
        config      : &Config,
        master_ip   : &String,
    ) -> Self {

        let backup_ip       : String            = config.elevator_ip_list[1].to_string() + ":" + &config.backup_port.to_string();
        // connect to backup. Will not continue until connection is established
        let mut backup_socket : Option<TcpStream> = None;
        while backup_socket.is_none() {
            match TcpStream::connect(&backup_ip) {
                Ok(stream) => {
                    backup_socket = Some(stream);
                    println!("[MASTER]\tConnected to backup");
                }
                Err(e) => {
                    println!("[MASTER]\tFailed to connect to backup: {}", e);
                    sleep(std::time::Duration::from_secs(1));
                }
            }
        } 
        
        
        // Create channels for backup
        let (master_to_backup_tx, master_to_backup_rx) = cbc::unbounded::<Message>();
        let (backup_to_master_tx, backup_to_master_rx) = cbc::unbounded::<Message>();
        
        
        // Spawn thread to handle backup connection
        let mut backup_stream = backup_socket.unwrap();
        let mut backup_stream_clone = backup_stream.try_clone().expect("Failed to clone backup stream");
        spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                backup_stream_clone.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
                match backup_stream_clone.read(&mut buffer) {
                    Ok(size) => {
                        if size > 0 {
                            let recieved: tcp::Message = bincode::deserialize(&buffer[..size]).expect("[MASTER]\tFailed to deserialize message from backup");
                            println!("[MASTER]\tReceived message from backup: {:#?}", recieved);
                            backup_to_master_tx.send(recieved).unwrap();
                        }
                    }
                    Err(e) => {
                        //eprintln!("[MASTER]\tFailed to recieve message from backup: {}", e)     
                        // Handle error here  
                    }
                }
            }
        });

        // Spawn thread to send messages to backup
        spawn(move || {
            loop {
                match master_to_backup_rx.recv() {
                    Ok(message) => {
                        let encoded = bincode::serialize(&message).expect("[MASTER]\tFailed to serialize message");
                        backup_stream.write(&encoded).expect("[MASTER]\tFailed to write to backup");  
                    }
                    Err(e) => {
                        println!("[MASTER]\tFailed to send message to backup: {}", e);
                        // Handle error here
                    }
                }
            }
        });

        
        Master {
            config                  : config.clone(),
            backup_to_master_rx,
            master_to_backup_tx,

        }
    }


    pub fn send_backup_data(&self, data: Message){
        self.master_to_backup_tx.send(data).unwrap();
    }

    pub fn master_loop(&mut self){
        let mut select: cbc::Select = cbc::Select::new();
        select.recv(&self.backup_to_master_rx);

        let recievers = vec![&self.backup_to_master_rx];
 
        loop {
            sleep(Duration::from_secs(3));
            
            self.master_to_backup_tx.send(Message::Idle(true)).unwrap();
            println!("[MASTER]\tSent idle message to backup");

            let index = select.ready();
            let result = recievers[index].try_recv();
            match result {
                Ok(message) => {
                    println!("[MASTER]\tReceived message from backup: {:#?}", message);
                    self.send_backup_data(message);
                }
                Err(_) => {}
            }
        }
    }
}




