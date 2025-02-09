use driver_rust::elevio;
use driver_rust::elevio::elev as e;

use crossbeam_channel as cbc;
use bincode;

use std::io::prelude::*;
use std::fmt::Display;
use std::net::TcpStream;
use std::thread::{spawn, sleep};
use std::time::Duration;

use crate::master;
use crate::{config::Config, inputs, tcp};  

#[derive(Debug, Clone, Copy)]
pub enum Direction {
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



// TODO: Kanskje en socket tilknyttet matster må være medlemsvariabel? 
#[derive(Debug)]
pub struct Slave {
    pub config                          : Config,
    pub elevator                        : e::Elevator,
    master_ip                           : String,
    nxt_order                           : u8,
    obstruction                         : bool,
    floor                               : usize,
    direction                           : Direction,
    behaviour                           : ElevatorBehaviour, 
    channels                            : inputs::SlaveChannels,                 // TODO: Kanskje dette bør være en del av slaven? så slipper vi å initiallisere den i main som Option
    master_socket                       : TcpStream,    
    //master_message                      : (cbc::Sender<tcp::Message>    , cbc::Receiver<tcp::Message>),
    door_timer                          : (cbc::Sender<bool>            , cbc::Receiver<bool>),
}


impl Slave {
    pub fn init(
            slave_addr          : String,     
            master_ip           : String,
            number_of_floors    : u8,
            config              : Config
        ) -> Slave
    {
        let conf                : Config                = config;
        let elev                : e::Elevator           = e::Elevator::init(&slave_addr, number_of_floors).expect("Failed to initialize elevator");
        let master_sckt         : TcpStream             = TcpStream::connect(master_ip.clone()).expect("Failed to connect to master");
        let chs                 : inputs::SlaveChannels = inputs::spawn_threads_for_slave_inputs(&elev, conf.input_poll_rate_ms.clone(), &master_sckt);
        let slave = Self {
            config              : conf,
            elevator            : elev,     
            master_ip           : master_ip,
            nxt_order           : 0,
            obstruction         : false,
            floor               : 0,
            direction           : Direction::Stop,
            behaviour           : ElevatorBehaviour::Idle,
            channels            : chs,
            master_socket       : master_sckt,           
            door_timer          : cbc::unbounded::<bool>(),
        };
        return slave;
    }

    // starter en egen tråd som sender beskjed når tidsuret for døren løper ut
    pub fn start_door_timer(&self, duration: Duration) {
        let tx = self.door_timer.0.clone();
        spawn(move || {
            sleep(duration);
            let _ =  tx.send(true).unwrap();
        });
    }

    pub fn send_new_cab_order(&mut self, cab_order: u8) {    
        let message = tcp::Message::NewOrder(cab_order);
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_)    => println!("[SLAVE]\tSent cab order: {}", cab_order),
            Err(e)   => println!("[SLAVE]\tFailed to send cab order: {}", e),
        }
    }
        
    pub fn send_order_complete(&mut self) {
        let message = tcp::Message::OrderComplete;
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_)    => println!("[SLAVE]\tSent order complete"),
            Err(e)   => println!("[SLAVE]\tFailed to send order complete: {}", e),
        }
    }
    
    pub fn send_stop_button(&mut self) {
        let message = tcp::Message::Error(tcp::ErrorState::EmergancyStop);
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_)           => println!("[SLAVE]\tSent stop button"),
            Err(e)          => println!("[SLAVE]\tFailed to send stop button: {}", e),
        }
    }
    
    // velger retning basert på neste ordre
    // TODO: fullfør denne funksjonen
    pub fn start_moving(&mut self) {
        
        /*         
            if self.floor == self.nxt_order as usize {
                self.direction = Direction::Stop;
                self.behaviour = ElevatorBehaviour::DoorOpen;
                }
                else if self.floor < self.nxt_order as usize {
                    self.direction = Direction::Up;
            self.behaviour = ElevatorBehaviour::Moving;
            }
            else {
                self.direction = Direction::Down;
                self.behaviour = ElevatorBehaviour::Moving;
                }
        */
        // setter heisen i bevegelse basert på self.direction
        match self.direction {
            Direction::Up   => self.elevator.motor_direction(e::DIRN_UP),
            Direction::Down => self.elevator.motor_direction(e::DIRN_DOWN),
            Direction::Stop => self.elevator.motor_direction(e::DIRN_STOP),
            }
    }

    // TODO! fullfør denne funksjonen
    pub fn slave_loop(&mut self) {
        loop {
            cbc::select! {
                // dersom channels er medlemsvariabel, kan vi fjerne unwrap
                recv(self.channels.floor_sensor_rx) -> msg => {
                    let floor_sensor = msg.unwrap();
                    println!("[SLAVE]\tReceived floor sensor message: {:#?}", floor_sensor);
                    self.floor = floor_sensor as usize;
                    
                    match self.behaviour {
                        ElevatorBehaviour::Moving => {
                            if self.floor == self.nxt_order as usize {
                                self.direction = Direction::Stop;
                                self.behaviour = ElevatorBehaviour::DoorOpen;
                                self.elevator.motor_direction(e::DIRN_STOP);
                                self.elevator.door_light(true); 
                                self.start_door_timer(Duration::from_secs(3));                          // starting doortimer
                                self.send_order_complete();                                             // Send order complete message to master
                            }
                        },
                        _ => {},                                                                        // Hvis heisen ikke er i bevegelse, gjør ingenting
                    }
                }

                // Receive call buttons from elevator
                recv(self.channels.call_button_rx) -> msg => {
                    let call_button = msg.unwrap();
                    println!("[SLAVE]\tReceived call button message: {:#?}", call_button);
                    
                    match call_button.call {
                        0 => {}                                             // Do nothing for HALL_UP
                        1 => {}                                             // Do nothing for HALL_DOWN
                        2 => self.send_new_cab_order(call_button.floor),    // Send new cab order
                        _ => panic!("Mottok ukjent knappetype"),
                    }
            
                    match self.behaviour {
                        ElevatorBehaviour::Idle => {
                            self.start_moving();
                        },
                        _ => {},
                    }
                }

                // Receive stop button from elevator
                recv(self.channels.stop_button_rx) -> msg => {
                    let stop_button = msg.unwrap();
                    println!("Stop button: {:#?}", stop_button);
                    self.elevator.motor_direction(e::DIRN_STOP);
                    self.behaviour = ElevatorBehaviour::OutOfOrder; 
                    self.send_stop_button();
                }
                
                // Receive obstruction from elevator
                recv(self.channels.obstruction_rx) -> msg => {
                    let obstr = msg.unwrap();
                    self.obstruction = obstr;
                    println!("Obstruction: {:#?}", obstr);
                }

                // Receive door timer expiration from door_timer
                recv(self.door_timer.1) -> _msg => {
                    if self.obstruction {
                        //println!("Obstruction detected. Timer reset.");
                        self.start_door_timer(Duration::from_secs(3));
                    }
                    else {
                        println!("Timer expired. Door closing.");
                        self.elevator.door_light(false);
                        self.start_moving();
                    }
                }

                // Receive incoming message from master
                recv(self.channels.master_message_rx) -> msg => {
                    match msg.unwrap() {
                        tcp::Message::NewOrder(order) => {
                            self.nxt_order = order;
                            println!("[SLAVE]\tReceived new order: {:#?}", order);
                        }
                        tcp::Message::OrderComplete => {}   // Do nothing for order complete message
                        tcp::Message::Error(_) => { println!("[SLAVE]\tReceived error message from master"); }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl Display for Slave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Slave {{
    config: {:?},
    elevator: {:?},
    master_ip: {},
    nxt_order: {},
    obstruction: {},
    floor: {},
    direction: {:?},
    behaviour: {:?},
    channels: {:?},
    master_socket: {:?},
    door_timer: {:?}
}}",
            self.config,
            self.elevator,
            self.master_ip,
            self.nxt_order,
            self.obstruction,
            self.floor,
            self.direction,
            self.behaviour,
            self.channels,
            self.master_socket,
            self.door_timer
        )
    }
}

