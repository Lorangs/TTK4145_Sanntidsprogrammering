#![allow(warnings)]

use driver_rust::elevio;
use driver_rust::elevio::elev as e;

use crossbeam_channel as cbc;
use bincode;

use std::io::{Write, prelude, Result};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::{TcpStream, SocketAddr, IpAddr, Ipv4Addr};
use std::str::FromStr;
use std::thread::{spawn, sleep};
use std::time::Duration;


use crate::config::Config;
use crate::inputs;
use crate::tcp; 

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
    nxt_order                           : u8,
    floor                               : u8,
    obstruction                         : bool,
    direction                           : Direction,
    behaviour                           : ElevatorBehaviour, 
    channels                            : inputs::SlaveChannels,                 
    master_socket                       : TcpStream,    
    door_timer                          : (cbc::Sender<bool>            , cbc::Receiver<bool>),
    light_matrix                        : Vec<[bool; 3]>,
}


impl Slave {
    pub fn init(
            slave_addr          : String,     
            config              : &Config
        ) -> Slave
    {
        let conf                : Config                = config.clone();
        let elev                : e::Elevator           = e::Elevator::init(&slave_addr, config.number_of_floors).expect("Failed to initialize elevator");
        let master_socket_addr  : SocketAddr            = SocketAddr::new(IpAddr::V4(Ipv4Addr::from_str(&config.elevator_ip_list[0]).unwrap()), config.master_port);
        let master_sckt         : TcpStream             = TcpStream::connect("127.0.0.1:4000").expect("Failed to connect to master");
                                                            //TcpStream::connect_timeout(&master_socket_addr,Duration::from_millis(config.tcp_timeout_ms)).expect("Failed to connect to master");
        let chs                 : inputs::SlaveChannels = inputs::spawn_threads_for_slave_inputs(&elev, conf.input_poll_rate_ms.clone(), &master_sckt);
        let mut slave = Self {
            config              : conf,
            elevator            : elev,     
            nxt_order           : 0,
            obstruction         : false,
            floor               : 0,
            direction           : Direction::Stop,
            behaviour           : ElevatorBehaviour::Idle,
            channels            : chs,
            master_socket       : master_sckt,           
            door_timer          : cbc::unbounded::<bool>(),
            light_matrix        : vec![[false; 3]; config.number_of_floors as usize],
        };


        // Initiate elevator position and lights
        slave.sync_lights();
        slave.elevator.door_light(false);
        slave.behaviour = ElevatorBehaviour::Moving;
        slave.direction = Direction::Down;
        slave.elevator.motor_direction(e::DIRN_DOWN);
        
        
        loop {
            cbc::select! {
                recv(slave.channels.floor_sensor_rx) -> msg => {
                    let floor_sensor = msg.unwrap();
                    println!("Received floor sensor message: {:#?}", floor_sensor);
                    slave.floor = floor_sensor;
                    if slave.floor !=u8::MAX{
                        slave.elevator.motor_direction(e::DIRN_STOP);
                        slave.direction = Direction::Stop;
                        slave.behaviour = ElevatorBehaviour::Idle;
                        slave.elevator.floor_indicator(slave.floor as u8);
                        break;
                    }
                }
            }
        }


        println!("[SLAVE]\tInitialized slave:\n{}", slave);
        return slave;
    }


    pub fn sync_lights(&self) {
        for (floor_index, light_array) in self.light_matrix.iter().enumerate() {
            let floor = floor_index as u8;
            self.elevator.call_button_light(floor, e::HALL_UP,    light_array[0]);
            self.elevator.call_button_light(floor, e::HALL_DOWN,  light_array[1]);
            self.elevator.call_button_light(floor, e::CAB,        light_array[2]);
        }
    }

    // starter en egen tråd som sender beskjed når tidsuret for døren løper ut
    pub fn start_door_timer(&self, duration: Duration) {
        let tx = self.door_timer.0.clone();
        spawn(move || {
            sleep(duration);
            let _ =  tx.send(true).unwrap();
        });
    }

    pub fn send_new_order(&mut self, floor: u8, button_type: u8) -> Result<()> {    
        let message = tcp::Message::NewOrder(floor, button_type);
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_)           => { 
                println!("[SLAVE]\tSent order:\nFloor:\t{}\nButton Type:\t{}", floor, button_type);    
                return Ok(()); 
            }
            Err(e)   => { 
                println!("[SLAVE]\tFailed to send cab order: {}", e); 
                return Err(e);
            }
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
            Err(e)   => println!("[SLAVE]\tFailed to send stop button: {}", e),
        }
    }
    
    // velger retning basert på neste ordre
    // TODO: fullfør denne funksjonen
    pub fn start_moving(&mut self) {
        if self.behaviour == ElevatorBehaviour::DoorOpen    ||
           self.behaviour == ElevatorBehaviour::OutOfOrder 
        {
            return;
        }

        if self.floor == self.nxt_order {
            self.direction = Direction::Stop;
            self.behaviour = ElevatorBehaviour::Idle;
        }
        else if self.floor > self.nxt_order {
            self.direction = Direction::Down;
            self.behaviour = ElevatorBehaviour::Moving;
        }
        else {
            self.direction = Direction::Up;
            self.behaviour = ElevatorBehaviour::Moving;
        }
        match self.direction {
            Direction::Stop => self.elevator.motor_direction(e::DIRN_STOP),
            Direction::Down => self.elevator.motor_direction(e::DIRN_DOWN),
            Direction::Up   => self.elevator.motor_direction(e::DIRN_UP),
        }
    }
    

    // TODO! fullfør denne funksjonen
    pub fn slave_loop(&mut self) {
        loop {
            cbc::select! {

                // Receive floor sensor from elevator
                recv(self.channels.floor_sensor_rx) -> msg => {
                    let floor_sensor = msg.unwrap();
                    println!("[SLAVE]\tReceived floor sensor message:\t{:#?}", floor_sensor);
                    self.floor = floor_sensor;
                    
                    match self.behaviour {
                        ElevatorBehaviour::Moving => {
                            // If the elevator is moving, check if it has reached the next order. If not: keep moving.
                            if self.floor == self.nxt_order{
                                self.direction = Direction::Stop;
                                self.elevator.motor_direction(e::DIRN_STOP);
                                self.behaviour = ElevatorBehaviour::DoorOpen;
                                self.elevator.door_light(true); 
                                self.start_door_timer(Duration::from_secs(3));                // starting doortimer
                                self.send_order_complete();                                   // Send order complete message to master
                            }
                        },
                        _ => {},                                                              // Hvis heisen ikke er i bevegelse, gjør ingenting
                    }
                }

                // Receive call buttons from elevator
                recv(self.channels.call_button_rx) -> msg => {
                    let call_button = msg.unwrap();
                    println!("[SLAVE]\t\tReceived call button message: {:#?}", call_button);
                    
                    // send new order to master
                    match self.send_new_order(call_button.floor, call_button.call) {
                        Ok(_)   => println!("[SLAVE]\t\tSent NewOrder"),
                        Err(e)  => println!("[SLAVE]\t\tFailed to send new order: {}", e),
                    }
                }

                // Receive stop button from elevator
                recv(self.channels.stop_button_rx) -> msg => {
                    let stop_button = msg.unwrap();
                    println!("[SLAVE]\t\tStop button: {:#?}", stop_button);
                    self.elevator.motor_direction(e::DIRN_STOP);
                    self.behaviour = ElevatorBehaviour::OutOfOrder; 
                    self.send_stop_button();
                }
                
                // Receive obstruction from elevator
                recv(self.channels.obstruction_rx) -> msg => {
                    let obstr = msg.unwrap();
                    self.obstruction = obstr;
                    println!("[SLAVE]\t\tObstruction: {:#?}", obstr);
                }

                // Receive door timer expiration from door_timer
                recv(self.door_timer.1) -> _msg => {
                    if self.obstruction {
                        //println!("Obstruction detected. Timer reset.");
                        self.start_door_timer(Duration::from_secs(3));
                    }
                    else {
                        println!("[SLAVE]\t\tTimer expired. Door closing.");
                        self.elevator.door_light(false);
                        self.start_moving();
                    }
                }

                // Receive incoming message from master
                recv(self.channels.master_message_rx) -> msg => {
                    let message = msg.unwrap();
                    match message {
                        tcp::Message::NewOrder(floor, _button_type) => {
                            self.nxt_order = floor;
                            println!("[SLAVE]\t\tReceived new order: {:#?}", floor);
                            self.start_moving();
                        },
                        tcp::Message::OrderComplete => {},   // Do nothing for order complete message
                        tcp::Message::LightMatrix(matrix) => {
                            self.light_matrix = matrix;
                            self.sync_lights();
                        },
                        tcp::Message::Error(_) => { println!("[SLAVE]\t\tReceived error message from master"); },
                        _ => {},
                    }
                }
            }
        }
    }
}

impl Display for Slave {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f, 
            "\tElevator:\t{:#?}\n\
            \tNxt_order:\t{:#?}\n\
            \tObstruction:\t{:#?}\n\
            \tFloor:\t\t{:#?}\n\
            \tDirection:\t{:#?}\n\
            \tBehaviour:\t{:#?}\n\
            \tChannels:\t{:#?}\n\
            \tMaster_socket:\t{:#?}\n\
            \tDoor_timer:\t{:#?}",
            
            self.elevator,
            //self.master_ip,
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

