use std::time::Duration
use std::String

pub enum Dirn {
    D_Down = -1,
    D_Stop = 0,
    D_Up = 1
}

pub enum Button {
    B_HallUp,
    B_HallDown,
    B_Cab
}

pub enum ClearOrderVariant{
    CV_All,
    CV_InDirn,
}

#[derive(Debug)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    OutOfOrder,
}



#[derive(Debug, Clone)]
pub struct Order {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}


pub struct Config {
    clear_request_variant   : ClearRequestVariant,
    door_open_duration_s    : Duration,
    ip_address              : String,
    num_floors              : u8,   
    poll_period_ms          : Duration,
}
