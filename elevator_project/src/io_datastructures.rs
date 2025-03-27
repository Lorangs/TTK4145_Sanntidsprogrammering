use crate::config::NUMBER_OF_FLOORS;
use crate::master::OrderRequests;
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    OutOfOrder,
}
impl ElevatorBehaviour{
    pub fn to_hall_assigner_lowercase(self) -> &'static str{
        match self {
            ElevatorBehaviour::Idle         => "idle",
            ElevatorBehaviour::Moving       => "moving",
            ElevatorBehaviour::DoorOpen     => "doorOpen",
            ElevatorBehaviour::OutOfOrder   => "outOfOrder",
        }
    }
}
impl FmtDisplay for ElevatorBehaviour {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                ElevatorBehaviour::Idle        => "Idle",
                ElevatorBehaviour::Moving      => "Moving",
                ElevatorBehaviour::DoorOpen    => "DoorOpen",
                ElevatorBehaviour::OutOfOrder  => "OutOfOrder",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Down = -1,
    Stop = 0,
    Up = 1,
}
impl Direction {
    pub fn to_hall_assigner_lowercase(self) -> &'static str{
        match self {
            Direction::Down => "down",
            Direction::Stop => "stop",
            Direction::Up => "up",
        }
    }
}
impl FmtDisplay for Direction {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                Direction::Down => "Down",
                Direction::Stop => "Stop",
                Direction::Up   => "Up",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElevatorState {
    pub behaviour: ElevatorBehaviour,
    pub floor: u8,
    pub direction: Direction,
    pub cab_requests: [bool; NUMBER_OF_FLOORS],
}
impl ElevatorState {

    /// Initialize ElevatorState to default values
    pub fn init() -> ElevatorState {
        ElevatorState {
            behaviour: ElevatorBehaviour::OutOfOrder,
            floor: 0,
            direction: Direction::Stop,
            cab_requests: [false; NUMBER_OF_FLOORS],
        }
    }
}
impl FmtDisplay for ElevatorState {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f,
            "ElevatorState:\n\t
            Behaviour:\t{}\n\t
            Floor:\t\t{}\n\t
            Directoin:\t{}\n\t
            CabRequests:\t{:?}",
            self.behaviour, self.floor, self.direction, self.cab_requests
        )
    }
}