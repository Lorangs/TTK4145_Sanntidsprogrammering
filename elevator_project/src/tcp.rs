// This file contains the TCP module, which is responsible for handling the TCP connection between the elevator and the scheduler.
use std::fmt;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Message{
    NewOrder(u8),
    OrderComplete,                    
    Error(ErrorState),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::NewOrder(id) => write!(f, "New order: {}", id),
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