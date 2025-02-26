// This file contains the TCP module, which is responsible for handling the TCP connection between the elevator and the scheduler.
use std::fmt;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct CallButton {
    pub floor: u8,          
    pub call: u8,           // 0: UP, 1: DOWN, 2: CAB
}

impl fmt::Display for CallButton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Floor: {}, Call: {}", self.floor, self.call)
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub enum Message{
    NewOrder(CallButton),               
    OrderComplete(CallButton),                    
    LightMatrix(Vec<[bool; 3]>),        // Hall_UP, Hall_DOWN, CAB_CALL for each floor
    Error(ErrorState),
}


impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::NewOrder( CallButton) => write!(f, "New Order: {}", CallButton),
            Message::OrderComplete ( CallButton) => write!(f, "Order complete: {}", CallButton),
            Message::LightMatrix(matrix) => {
                // Hall_UP, Hall_DOWN, CAB_CALL
                write!(f, "HU\tHD\tCAB\n");
                for i in 0..matrix.len() 
                {
                    write!(f, "{}\t{}\t{}\n", matrix[i][0], matrix[i][1], matrix[i][2]);
                }
                Ok(())
            }
            Message::Error(id) => write!(f, "Error: {}", id),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorState {
    OK,
    EmergancyStop,
    DoorObstruction,
    Network(String),
}

impl fmt::Display for ErrorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorState::OK => write!(f, "OK"),
            ErrorState::EmergancyStop => write!(f, "Emergancy stop"),
            ErrorState::DoorObstruction => write!(f, "Door obstruction"),
            ErrorState::Network(s) => write!(f, "Network error: {}", s),
        }
    }
}