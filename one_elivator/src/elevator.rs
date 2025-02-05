use std::io::*;
use std::string::String;
use std::time::Duration;
use crossbeam_channel as cbc;
use driver_rust::elevio::elev as e;

use crate::timer;
const NUMBER_OF_FLOORS: usize = 4;

#[derive(Debug, Clone, Copy)]
pub enum Dirn {
    Down = -1,
    Stop = 0,
    Up = 1
}

#[derive(Debug, Clone, Copy)]
pub enum ClearOrderVariant{
    CvAll,
    CvInDirn,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    OutOfOrder,
}



#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub clear_request_variant   : ClearOrderVariant,
    pub ip_address              : String,
    pub number_of_floors        : usize,   
    pub poll_period_ms          : Duration,
}


#[derive(Debug, Clone)]
pub struct Slave {
    pub elevator        : e::Elevator,
    pub floor           : usize,
    pub direction       : Dirn,
    pub behaviour       : ElevatorBehaviour, 
    pub orders          : [Order; NUMBER_OF_FLOORS as usize],
    pub config          : Config,
    pub obstruction     : bool,
}

impl Slave {
    pub fn init(addr: String) -> Result<Slave> {
        Ok( Self {
                elevator    : e::Elevator::init(&addr, NUMBER_OF_FLOORS as u8)?,
                floor       : usize::MAX,                                   // initialisering setter floor til -1
                direction   : Dirn::Stop,
                behaviour   : ElevatorBehaviour::Idle,
                orders      : [Order{
                                    hall_down   : false,
                                    hall_up     : false,
                                    cab_call    : false,    
                                }; NUMBER_OF_FLOORS as usize],
                config      : Config{ 
                                clear_request_variant   : ClearOrderVariant::CvAll,
                                ip_address              : addr,
                                poll_period_ms          : Duration::from_millis(25),
                                number_of_floors        : NUMBER_OF_FLOORS,
                            },
                obstruction : false,
        })
    }

    
    pub fn orders_above(&self) -> bool{
        for floor in (self.floor + 1) .. self.config.number_of_floors {
            if self.orders[floor].hall_down || self.orders[floor].hall_up || self.orders[floor].cab_call {
                return true;
            }
        }
        return false;   
    }

    pub fn orders_below(&self) -> bool {
        for floor in 0 .. self.floor {
            if self.orders[floor].hall_down || self.orders[floor].hall_up || self.orders[floor].cab_call {
                return true;
            }  
        }
        return false;
    }

    pub fn orders_here(&self) -> bool {
        return 
            self.orders[self.floor].hall_down  || 
            self.orders[self.floor].hall_up    || 
            self.orders[self.floor].cab_call;
    }

    pub fn should_stop(&self) -> bool{
        match self.direction{
            Dirn::Down => {
                self.orders[self.floor].hall_down ||
                self.orders[self.floor].cab_call  ||
                !self.orders_below()
            }
            Dirn::Up => {
                self.orders[self.floor].hall_up   ||
                self.orders[self.floor].cab_call  ||
                !self.orders_above()
            }
            _=> true
        }
    }

    pub fn choose_direction(&self) -> (Dirn, ElevatorBehaviour) {
        match &self.direction {
            Dirn::Up => { return
                if      self.orders_above() { ( Dirn::Up,   ElevatorBehaviour::Moving ) }
                else if self.orders_here()  { ( Dirn::Down, ElevatorBehaviour::DoorOpen ) }
                else if self.orders_below() { ( Dirn::Down, ElevatorBehaviour::Moving ) }
                else                        { ( Dirn::Stop, ElevatorBehaviour::Idle ) }
            }

            Dirn::Down => { return 
                if      self.orders_below() { ( Dirn::Down, ElevatorBehaviour::Moving ) }
                else if self.orders_here()  { ( Dirn::Up,   ElevatorBehaviour::DoorOpen ) }
                else if self.orders_above() { ( Dirn::Up,   ElevatorBehaviour::Moving ) }
                else                        { ( Dirn::Stop, ElevatorBehaviour::Idle ) }
            }

            Dirn::Stop => { return 
                if      self.orders_here()  { ( Dirn::Stop, ElevatorBehaviour::DoorOpen ) }
                else if self.orders_above() { ( Dirn::Up,   ElevatorBehaviour::Moving ) }
                else if self.orders_below() { ( Dirn::Down, ElevatorBehaviour::Moving ) }
                else                        { ( Dirn::Stop, ElevatorBehaviour::Idle ) }
            }
        }
    }

    pub fn clear_at_current_floor(&mut self) {
        self.orders[self.floor].cab_call    = false;
        self.orders[self.floor].hall_down   = false;
        self.orders[self.floor].hall_up     = false;
    }

    pub fn start_moving(&mut self, timer_tx: &cbc::Sender<bool>) {
        let (diraction, behaviour) = self.choose_direction();
        self.behaviour = behaviour;

        if behaviour == ElevatorBehaviour::DoorOpen {
            println!("Stopped with door open at floor {:?}", self.floor);
            self.clear_at_current_floor();
            timer::start_timer(&timer_tx, std::time::Duration::from_secs(3));
        }

        match diraction {
            Dirn::Up   => {
                self.elevator.motor_direction(e::DIRN_UP);
                self.direction = Dirn::Up;
            },
            Dirn::Down => {
                self.elevator.motor_direction(e::DIRN_DOWN);
                self.direction = Dirn::Down;
            },
            Dirn::Stop => {
                self.elevator.motor_direction(e::DIRN_STOP);
                self.direction = Dirn::Stop;
            },
        }
    }

    pub fn sync_lights(&self) {
        for (floor, order) in self.orders.iter().enumerate() {
            let floor = floor as u8;
            self.elevator.call_button_light(floor, e::HALL_UP,    order.hall_up);
            self.elevator.call_button_light(floor, e::HALL_DOWN,  order.hall_down);
            self.elevator.call_button_light(floor, e::CAB,        order.cab_call);
        }
    }
}   





 /*   
fn requests_shouldClearImmediately(e: Elevator, btn_floor:u8, btn_type:Button)-> bool{
    match e.config.clearRequestVariant{

        CV_All      =>  e.floor == btn_floor;
        CV_InDirn   =>  e.floor == btn_floor && 
                        (
                            (e.dirn == D_Up   && btn_type == B_HallUp)    ||
                            (e.dirn == D_Down && btn_type == B_HallDown)  ||
                            e.dirn == D_Stop ||
                            btn_type == B_Cab
                        );  
                
        _=> false
    }
}



fn elevator_print(e: Elevator){
    println!("  +--------------------+
    \n  |floor = {} |
    \n  |dirn  = {} |
    \n  |behav = {} |",
    e.floor,
    elevio_dirn_toString(es.dirn),
    eb_toString(e.behaviour)
);
println!("  +--------------------+\n");
println!("  |  | up  | dn  | cab |\n");
for(int floor = N_FLOORS-1; floor >= 0; floor--){
    println!("  | {}", floor);
        for(int btn = 0; btn < N_BUTTONS; btn++){
            if((floor == N_FLOORS-1 && btn == B_HallUp)  || 
            (floor == 0 && btn == B_HallDown) 
        ){
            println!("|     ");
        } else {
            println!(e.requests[floor][btn] ? "|  #  " : "|  -  ");
        }
    }
    println!("|\n")pub enum ClearRequestVariant{
            CV_All,
            CV_InDirn,
        };
    }
    println!("  +--------------------+\n");
}
*/
