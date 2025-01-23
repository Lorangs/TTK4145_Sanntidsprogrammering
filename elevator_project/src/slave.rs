use driver_rust::elevio;
use driver_rust::elevio::elev as e;

use std::io::*;


struct Slave {
    elevator: e::Elevator,
    mut master_ip : String,
    mut nxt_order : u8,
};


impl Slave {

    pub fn init() 

    pub fn move_elevator ()

    pub fn recieve_order ()
    
    pub fn send_order ()

    pub fn recieve_master_id ()


}