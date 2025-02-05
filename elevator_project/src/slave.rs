use driver_rust::elevio;
use driver_rust::elevio::elev as e;
use crossbeam_channel as cbc;

use crate::{inputs, door_timer};  

#[derive(Debug, Clone, Copy)]
pub enum Dirn {
    Down = -1,
    Stop = 0,
    Up = 1
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    OutOfOrder,
}

struct Slave {
    elevator                            : e::Elevator,
    master_ip                           : String,
    nxt_order                           : u8,
    obstruction                         : bool,
    floor                               : usize,
    direction                           : Dirn,
    behaviour                           : ElevatorBehaviour, 
    channels                            : inputs::Channels,
    (door_timer_tx, door_timer_rx)      : (cbc::Sender<bool>, cbc::Receiver<bool>),
}


impl Slave {
    pub fn init(
            slave_addr          : String,     
            master_ip           : String,
            number_of_floors    : u8
        ) -> Result<Slave> 
    {
        Ok( Self {
            elevator                            : e::Elevator::init(&slave_addr, number_of_floors)?;
            master_ip                           : master_ip,
            nxt_order                           : 0,
            obstruction                         : false,
            floor                               : 0,
            direction                           : Dirn::Stop,
            behaviour                           : ElevatorBehaviour::Idle,
            channels                            : inputs::get_channels(&Self),                      // Usikker på om dette er riktig
            (door_timer_tx, door_timer_rx)      : cbc::unbounded::<bool>(),
        })
    }

    pub fn send_new_cab_order(&self, floor: u8) -> Result<()>  {}

    pub fn send_order_complete(&self) -> Result<()> {}

    pub fn send_stop_button(&self) -> Result<()> {}

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

    pub fn start_moving(&mut self) {
        (self.diraction, self.behaviour) = self.choose_direction();
        
        match self.diraction {
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

    pub fn slave_loop(&self) -> Result<()> {
        loop {
            cbc::select! {

                recv(self.channels.floor_sensor_rx) -> msg => {
                    let floor_sensor = msg.unwrap();
                    println!("[SLAVE]\tReceived floor sensor message: {:#?}"floor_sensor);
                    self.floor = floor_sensor as usize;
                    
                    match self.behaviour {
                        ElevatorBehaviour::Moving => {
                            if self.floor == nxt_order {
                                self.elevator.motor_direction(e::DIRN_STOP);
                                self.direction = Dirn::Stop;
                                self.behaviour = ElevatorBehaviour::DoorOpen;
                                self.elevator.door_light(true);
                                self.send_order_complete();
                                door_timer::start_timer(self.door_timer_tx, Duration::from_secs(3));    // starting doortimer
                            }
                        },
                        _ => {},
                    }
                }

                recv(self.channels.call_button_rx) -> msg => {
                    let call_button = msg.unwrap();
                    println!("[SLAVE]\tReceived call button message: {:#?}", call_button);

                    match call_button.call {
                        0 => {}                                     // Do nothing for HALL_UP
                        1 => {}                                     // Do nothing for HALL_DOWN
                        2 => self.send_new_cab_order(call_button.floor), // Send new cab order
                        _ => panic!("Mottok ukjent knappetype"),
                    }

                    
                    match self.behaviour {
                        ElevatorBehaviour::Idle => {
                            self.start_moving(&timer_tx);
                        },
                        _ => {},
                    }
                }

                // Receive stop button message
                recv(self.channels.stop_button_rx) -> msg => {
                    let stop_button = msg.unwrap();
                    println!("Stop button: {:#?}", stop_button);
                    self.elevator.motor_direction(e::DIRN_STOP);
                    self.behaviour = ElevatorBehaviour::OutOfOrder; 
                    self.send_stop_button();
                }

                // Receive obstruction message
                recv(self.channels.obstruction_rx) -> msg => {
                    let obstr = msg.unwrap();
                    self.obstruction = obstr;
                    println!("Obstruction: {:#?}", obstr);
                }

                // Receive timer message
                recv(self.door_timer_rx) -> _msg => {
                    if self.obstruction {
                        //println!("Obstruction detected. Timer reset.");
                        door_timer::start_timer(self.door_timer_tx, Duration::from_secs(3));
                    }
                    else {
                        println!("Timer expired. Door closing.");
                        self.elevator.door_light(false);
                        self.start_moving();
                    }
                }
            }
        }
    }
}
