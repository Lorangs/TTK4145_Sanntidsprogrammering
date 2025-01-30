use std::io::*;
use std::string::String;
use std::time::Duration;

use driver_rust::elevio::elev as e;

use crate::timer::Timer;
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
    pub dirn            : Dirn,
    pub behaviour       : ElevatorBehaviour,
    pub door_timer      : Timer, 
    pub orders          : [Order; NUMBER_OF_FLOORS],
    pub config          : Config,
}

impl Slave {
    pub fn init(addr: String) -> Result<Slave> {
        Ok( Self {
                elevator    : e::Elevator::init(&addr, NUMBER_OF_FLOORS as u8)?,
                floor       : usize::MAX,                                       // initialisering setter floor til usize::MAX
                dirn        : Dirn::Stop,
                behaviour   : ElevatorBehaviour::Idle,
                door_timer  : Timer::start(Duration::from_secs(3)),
                orders      : [Order{
                                    hall_down   : false,
                                    hall_up     : false,
                                    cab_call    : false,    
                                }; NUMBER_OF_FLOORS],
                config      : Config{ 
                                clear_request_variant   : ClearOrderVariant::CvAll,
                                ip_address              : addr,
                                poll_period_ms          : Duration::from_millis(25),
                                number_of_floors        : NUMBER_OF_FLOORS,
                            },
        })
    }
    //må skjekke om for loopane har rett range, kan ver det skal ver eks:N_FLOORS-1 
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
        match self.dirn{
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

    pub fn choose_dirn(&self) -> (Dirn, ElevatorBehaviour) {
        match &self.dirn {
            Dirn::Up => { 
                if      self.orders_above() { ( Dirn::Up,   ElevatorBehaviour::Moving ) }
                else if self.orders_below() { ( Dirn::Down, ElevatorBehaviour::Moving ) }
                else if self.orders_here()  { ( Dirn::Down, ElevatorBehaviour::DoorOpen ) }
                else                        { ( Dirn::Stop, ElevatorBehaviour::Idle ) }
            }

            Dirn::Down => {
                if      self.orders_below() { ( Dirn::Down, ElevatorBehaviour::Moving ) }
                else if self.orders_here()  { ( Dirn::Up,   ElevatorBehaviour::DoorOpen ) }
                else if self.orders_above() { ( Dirn::Up,   ElevatorBehaviour::Moving ) }
                else                        { ( Dirn::Stop, ElevatorBehaviour::Idle ) }
            }

            Dirn::Stop => {
                if      self.orders_here()  { ( Dirn::Stop, ElevatorBehaviour::DoorOpen ) }
                else if self.orders_above() { ( Dirn::Up,   ElevatorBehaviour::Moving ) }
                else if self.orders_below() { ( Dirn::Down, ElevatorBehaviour::Moving ) }
                else                        { ( Dirn::Stop, ElevatorBehaviour::Idle ) }
            }
            
            _=> ( Dirn::Stop, ElevatorBehaviour::Idle )
        }
    }

    pub fn clear_at_current_floor(&mut self) {
        self.orders[self.floor].cab_call    = false;
        self.orders[self.floor].hall_down   = false;
        self.orders[self.floor].hall_up     = false;
    }

    pub fn start_moving(&mut self) {
        let (diraction, behaviour) = self.choose_dirn();
        self.behaviour = behaviour;

        if behaviour == ElevatorBehaviour::DoorOpen {
            println!("Stopping in move!");
            self.clear_at_current_floor();
            self.door_timer = Timer::start(Duration::from_secs(3)); // TODO usikker på om doortimer funkerer som tiltenkt.
        }

        match diraction {
            Dirn::Up => {
                self.elevator.motor_direction(e::DIRN_UP);
            }
            Dirn::Down => {
                self.elevator.motor_direction(e::DIRN_DOWN);
            }
            Dirn::Stop => {
                self.elevator.motor_direction(e::DIRN_STOP);
            }
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
