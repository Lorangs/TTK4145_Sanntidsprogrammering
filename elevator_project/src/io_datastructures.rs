use crate::config::NUMBER_OF_FLOORS;
use crate::master::OrderRequests;
use crate::slave::ElevatorState;
use serde::{Deserialize, Serialize};
use std::fmt::{Display as FmtDisplay, Formatter as FmtFormatter, Result as FmtResult};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct CallButton {
    pub floor: u8,
    pub call: u8, // 0: UP, 1: DOWN, 2: CAB
}

impl FmtDisplay for CallButton {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(f, "Floor: {}, Call: {}", self.floor, self.call)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    NewOrder(CallButton),
    OrderComplete(CallButton),
    StateUpdate(ElevatorState),
    LightMatrix([[bool; 2]; NUMBER_OF_FLOORS]), // Hall_UP, Hall_DOWN for each floor.
    Backup(OrderRequests),
    Error(ErrorState),
}

impl FmtDisplay for Message {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        match self {
            Message::NewOrder(call_button) => write!(f, "New Order: {}", call_button),
            Message::OrderComplete(call_button) => write!(f, "Order complete: {}", call_button),
            Message::StateUpdate(state) => write!(f, "State update: {}", state),
            Message::LightMatrix(_matrix) => write!(f, "Light matrix"),
            Message::Backup(b) => write!(f, "Backup: {:#?}", b),
            Message::Error(id) => write!(f, "Error: {}", id),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorState {
    EmergancyStop,
    Network,
}

impl FmtDisplay for ErrorState {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        match self {
            ErrorState::EmergancyStop => write!(f, "Emergancy stop"),
            ErrorState::Network => write!(f, "Network error"),
        }
    }
}

