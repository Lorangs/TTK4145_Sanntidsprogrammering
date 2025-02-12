// This file contains the TCP module, which is responsible for handling the TCP connection between the elevator and the scheduler.
use std::fmt;
use serde::{Serialize, Deserialize};

/* Button_type from driver_rust:
pub const HALL_UP   : u8 = 0;
pub const HALL_DOWN : u8 = 1;
pub const CAB       : u8 = 2;
 */

#[derive(Serialize, Deserialize, Debug)]
pub enum Message{
    NewOrder(u8, u8),           // Floor, Button_type
    OrderComplete,                    
    Error(ErrorState),
}



impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::NewOrder(floor, button_type) => write!(f, "New Order:\nFloor:\t{}\nCall:\t{}", floor, button_type),
            Message::OrderComplete => write!(f, "Order complete."),
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