// This file contains the TCP module, which is responsible for handling the TCP connection between the elevator and the scheduler.
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::master::MasterQueues;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct CallButton {
    pub floor: u8,
    pub call: u8, // 0: UP, 1: DOWN, 2: CAB
}

impl fmt::Display for CallButton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Floor: {}, Call: {}", self.floor, self.call)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    NewOrder(CallButton),
    OrderComplete(CallButton),
    LightMatrix(Vec<[bool; 3]>), // Hall_UP, Hall_DOWN, CAB_CALL for each floor. 
    Error(ErrorState),
    Backup(MasterQueues),
    Idle(bool),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Message::NewOrder(call_button) => write!(f, "New Order: {}", call_button),
            Message::OrderComplete(call_button) => write!(f, "Order complete: {}", call_button),
            Message::LightMatrix(matrix) => write!(f, "Light matrix: {:#?}", matrix),
            Message::Error(id) => write!(f, "Error: {}", id),
            Message::Backup(b) => write!(f, "Backup: {:#?}", b),
            Message::Idle(b) => write!(f, "Idle: {}", b),
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
