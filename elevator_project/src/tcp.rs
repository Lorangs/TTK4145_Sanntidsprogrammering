// This file contains the TCP module, which is responsible for handling the TCP connection between the elevator and the scheduler.

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Message{
    NewOrder(u8, u8),
    OrderComplete,                    
    Error(ErrorState),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorState {
    OK,
    EmergancyStop,
    DoorObstruction,
    Network(String),
}
