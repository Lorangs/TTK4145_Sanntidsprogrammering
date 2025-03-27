use crate::config::{NUMBER_OF_FLOORS, NUMBER_OF_ELEVATORS};
use driver_rust::elevio::elev::{CAB, HALL_DOWN, HALL_UP};
use debug_print::debug_println as dprintln;
use serde_json::{json, Map, Value};
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::process::Command;
use std::collections::HashMap;
use std::fmt::{Display as FmtDisplay, Formatter as FmtFormatter, Result as FmtResult};

/// Enum for messages sent over TCP between the different units.
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
            Message::NewOrder(call_button)      => write!(f, "New Order: {}", call_button),
            Message::OrderComplete(call_button) => write!(f, "Order complete: {}", call_button),
            Message::StateUpdate(state)      => write!(f, "State update: {}", state),
            Message::LightMatrix(_matrix)   => write!(f, "Light matrix"),
            Message::Backup(b)               => write!(f, "Backup: {:#?}", b),
            Message::Error(id)                  => write!(f, "Error: {}", id),
        }
    }
}

/// Struct for call buttons pushed by users.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct CallButton {
    pub floor: u8,
    pub call: u8, // 0: UP, 1: DOWN, 2: CAB
}
impl FmtDisplay for CallButton {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f, 
            "\tFloor:\t{}\n\
            \tCall:\t{}", 
            self.floor,
            self.call
        )
    }
}

/// Enum for custom error states.
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

/// Enum for the different behaviour the elevator can have.
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

/// Enum for the different directions the elevator can move.
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
            Direction::Up   => "up",
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

/// Struct for the state of the elevator. This include the behaviour, floor, direction and local cab requests.
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
            "ElevatorState:\n\
            \tBehaviour:\t{}\n\
            \tFloor:\t\t{}\n\
            \tDirectoin:\t{}\n\
            \tCabRequests:\t{:?}",
            self.behaviour, self.floor, self.direction, self.cab_requests
        )
    }
}

/// Struct for all requests from every connected slave.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderRequests {
    pub hall_requests: [[bool; 2]; NUMBER_OF_FLOORS],
    pub states: [ElevatorState; NUMBER_OF_ELEVATORS],
}

impl OrderRequests {

    /// Initialize the OrderRequests struct with empty hall requests and elevator states.
    pub fn init() -> OrderRequests {
        let hall_requests: [[bool; 2]; NUMBER_OF_FLOORS] = [[false; 2]; NUMBER_OF_FLOORS];
        let states: [ElevatorState; NUMBER_OF_ELEVATORS] = [ElevatorState::init(); NUMBER_OF_ELEVATORS];

        OrderRequests {
            hall_requests,
            states,
        }
    }

    /// Update the hall requests with a new call button. true for add, false for remove
    pub fn update_hall_requests(&mut self, call: CallButton, add_or_remove: bool) { 
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
                    "behaviour": state.behaviour.to_hall_assigner_lowercase(),
                    "direction": state.direction.to_hall_assigner_lowercase(),
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
    
    /// Serialize the OrderRequests struct to a JSON string.
    pub fn to_json_string(&self) -> String {
        match serde_json::to_string(&self){
            Ok(json) => json,
            Err(e) => {
                dprintln!("[MASTER]\tFailed to serialize OrderRequests to JSON: {}", e);
                dprintln!("[MASTER]\tReturning empty JSON string");
                String::new()
            }
        }
    }
}
impl FmtDisplay for OrderRequests {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f,
            "OrderRequests:\n\
            \tHall queue:\t{:?}\n\
            \tCab queues:\t{:?}",
            self.hall_requests, 
            self.states
        )
    }
}