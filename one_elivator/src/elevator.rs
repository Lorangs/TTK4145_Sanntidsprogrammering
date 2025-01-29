use std::fmt;
use std::io::*;
use std::String
use std::time::Duration
use driver_rust::elevio::*;
use driver_rust::elevio::elev as e;

mod elevator_io_types
use crate::elevator_io_types::*


const NUMBER_OF_FLOORS: u8 = 4;

#[derive(Debug, Clone)]
pub struct Slave {
    pub elevator        : e,
    pub floor:          : i8,
    pub dirn:           : Dirn,
    pub orders          : [Order; NUMBER_OF_FLOORS],
    pub config          : Config,
    pub behaviour       : ElevatorBehaviour  
}

impl Slave {
    pub fn init(addr: String, num_floors: u8) -> Result<Slave> {
        Ok( Self {
                elevator    : e::init(addr, num_floors),
                floor       : -1,     
                dirn        : D_Stop,
                behaviour   : Idle,
                orders      : [Order{
                                    hall_down   : false,
                                    hall_up     : false,
                                    cab_call    : false,    
                                }; NUMBER_OF_FLOORS],
                config:     { 
                                clear_request_variant   : CV_All,
                                door_open_duration_s    : Duration::from_secs(3),
                                ip_address              : addr,
                                poll_period_ms          : Duration::from_millis(25),
                                num_floors              : NUMBER_OF_FLOORS,
                            },
        })
    }
    //må skjekke om for loopane har rett range, kan ver det skal ver eks:N_FLOORS-1 
    pub fn orders_above(&self) -> bool{
        for floor:u8 in (self.floor + 1) .. self.config.num_floors {
            if (self.orders[floor].hall_down || self.orders[floor].hall_up || self.orders[floor].cab_call){
                return true;
            }
        }
        return false;   
    }

    pub fn ordres_below(&self) -> bool {
        for floor:u8 in 0 .. self.floor {
            if (self.orders[floor].hall_down || self.orders[floor].hall_up || self.orders[floor].cab_call){
                return true;
            }  
        }
        return false;
    }

    pub fn orders_here(&self) -> bool {
        (self.orders[self.floor].hall_down || self.orders[self.floor].hall_up || self.orders[self.floor].cab_call)
    }

    pub fn should_stop(&self) -> bool{
        match self.dirn{
            D_Down => {
                self.orders[self.floor].hall_down ||
                self.orders[self.floor].cab_call  ||
                !self.orders_below()
            }
            D_Up => {
                self.orders[self.floor].hall_up   ||
                self.orders[self.floor].cab_call  ||
                !self.orders_above()
            }
            _=> true
        }
    }

    pub fn choose_dirn(&self) -> (Dirn, ElevatorBehaviour) {
        match self.dirn {


            D_Up => { 
                if      self.orders_above() { ( D_Up,   Moving ) }
                else if self.orders_below() { ( D_Down, Moving ) }
                else if self.orders_here()  { ( D_Down, DoorOpen ) }
                else                        { ( D_Stop, Idle ) }
            }

            D_Down => {
                if      self.orders_below() { ( D_Down, Moving ) }
                else if self.orders_here()  { ( D_Up,   DoorOpen ) }
                else if self.orders_above() { ( D_Up,   Moving ) }
                else                        { ( D_Stop, Idle ) }
            }

            D_Stop => {
                if      self.orders_here()  { ( D_Stop, DoorOpen ) }
                else if self.orders_above() { ( D_Up,   Moving ) }
                else if self.orders_below() { ( D_Down, Moving ) }
                else                        { ( D_Stop, Idle ) }
            }
            
            _=> ( D_Stop, Idle )
        }
    }

    pub fn clear_at_current_floor(&self) {
        self.orders[self.floor].cab_call    = false;
        self.orders[self.floor].hall_down   = false;
        self.orders[self.floor].hall_up     = false;
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
