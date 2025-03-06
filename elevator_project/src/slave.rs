use driver_rust::elevio::elev as e;
use bincode;
use crossbeam_channel as cbc;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Result, Write};
use std::net::TcpStream;
use std::thread::{sleep, spawn};
use std::time::Duration;

use crate::config::Config;
use crate::inputs;
use crate::tcp;

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Down = -1,
    Stop = 0,
    Up = 1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    OutOfOrder,
}

// TODO: Maybe socket related to master should be a member variable?
#[derive(Debug)]
pub struct Slave {
    pub config: Config,
    pub elevator: e::Elevator,
    nxt_order: tcp::CallButton,
    floor: u8,
    obstruction: bool,
    direction: Direction,
    behaviour: ElevatorBehaviour,
    channels: inputs::SlaveChannels,
    master_socket: TcpStream,
    door_timer: (cbc::Sender<bool>, cbc::Receiver<bool>),
    light_matrix: Vec<[bool; 3]>, // Hall_UP, Hall_DOWN, CAB_CALL for each floor
}

impl Slave {
    pub fn init(slave_addr: String, config: &Config) -> Slave {
        let conf: Config = config.clone();
        let elev: e::Elevator = e::Elevator::init(&slave_addr, config.number_of_floors)
            .expect("Failed to initialize elevator");

        // TODO : Implement way to change master_ip and slave_ip dynamically. 
        let master_ip: String =
            config.elevator_ip_list[0].clone().to_string() + ":" + &config.master_port.to_string();
        let master_sckt: TcpStream =
            TcpStream::connect(master_ip).expect("Failed to connect to master");
        let chs: inputs::SlaveChannels = inputs::spawn_threads_for_slave_inputs(
            &elev,
            conf.input_poll_rate_ms.clone(),
            &master_sckt,
        );
        let mut slave = Self {
            config: conf,
            elevator: elev,
            nxt_order: tcp::CallButton { floor: 0, call: 0 },
            obstruction: false,
            floor: 0,
            direction: Direction::Stop,
            behaviour: ElevatorBehaviour::Idle,
            channels: chs,
            master_socket: master_sckt,
            door_timer: cbc::unbounded::<bool>(),
            light_matrix: vec![[false; 3]; config.number_of_floors as usize],
        };

        // Turns all lights off
        slave.sync_lights();
        slave.elevator.door_light(false);

        // Initiate elevator position and lights to the nearest floor in downwards direction
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

        println!("[SLAVE]\t\tInitialized slave:\n{}", slave);
        return slave;
    }

    // Poll light information from dirver and update light_matrix
    pub fn sync_lights(&self) {
        println!("Syncing lights");
        for (floor_index, light_array) in self.light_matrix.iter().enumerate() {
            let floor = floor_index as u8;
            self.elevator
                .call_button_light(floor, e::HALL_UP, light_array[0]);
            self.elevator
                .call_button_light(floor, e::HALL_DOWN, light_array[1]);
            self.elevator
                .call_button_light(floor, e::CAB, light_array[2]);
        }
    }

    // Spawn a new thread that will sleep for the given duration and then send a message to the door_timer channel when done. 
    pub fn start_door_timer(&self, duration: Duration) {
        let tx = self.door_timer.0.clone();
        spawn(move || {
            sleep(duration);
            let _ = tx.send(true).unwrap();
        });
    }

    pub fn send_new_order(&mut self, callbutton: tcp::CallButton) -> Result<()> {
        let message = tcp::Message::NewOrder(callbutton.clone());
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_) => {
                println!("[SLAVE]\t\tSent order:\t{}", callbutton);
                return Ok(());
            }
            Err(e) => {
                println!("[SLAVE]\t\tFailed to send cab order: {}", e);
                return Err(e);
            }
        }
    }

    pub fn send_order_complete(&mut self) {
        let message = tcp::Message::OrderComplete(self.nxt_order);
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_) => println!("[SLAVE]\t\tSent order complete"),
            Err(e) => println!("[SLAVE]\t\tFailed to send order complete: {}", e),
        }
    }

    pub fn send_stop_button(&mut self) {
        let message = tcp::Message::Error(tcp::ErrorState::EmergancyStop);
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_) => println!("[SLAVE]\t\tSent stop button"),
            Err(e) => println!("[SLAVE]\t\tFailed to send stop button: {}", e),
        }
    }
    pub fn send_idle(&mut self) {
        let message = tcp::Message::Idle(true);
        let encoded: Vec<u8> = bincode::serialize(&message).unwrap();
        match self.master_socket.write(&encoded) {
            Ok(_) => println!("[SLAVE]\t\tSent idle"),
            Err(e) => println!("[SLAVE]\t\tFailed to send idle: {}", e),
        }
    }

    // Choose direction based on next order and start moving. 
    // TODO: Is this completed?
    pub fn start_moving(&mut self) {
        if self.behaviour == ElevatorBehaviour::DoorOpen
            || self.behaviour == ElevatorBehaviour::OutOfOrder
        {
            // Do nothing if the elevator is out of order or the door is open
            return;
        }

        if self.floor == self.nxt_order.floor {
            self.direction = Direction::Stop;
            self.behaviour = ElevatorBehaviour::Idle;
        } else if self.floor > self.nxt_order.floor {
            self.direction = Direction::Down;
            self.behaviour = ElevatorBehaviour::Moving;
        } else {
            self.direction = Direction::Up;
            self.behaviour = ElevatorBehaviour::Moving;
        }
        match self.direction {
            Direction::Stop => self.elevator.motor_direction(e::DIRN_STOP),
            Direction::Down => self.elevator.motor_direction(e::DIRN_DOWN),
            Direction::Up => self.elevator.motor_direction(e::DIRN_UP),
        }
    }


    // State machine for the slave elevator
    pub fn slave_loop(&mut self) {
        loop {
            if self.behaviour == ElevatorBehaviour::Idle {
                self.send_idle();
            }
            cbc::select! {

                // Receive floor sensor from elevator
                recv(self.channels.floor_sensor_rx) -> msg => {
                    let floor_sensor = msg.unwrap();
                    self.floor = floor_sensor;
                    self.elevator.floor_indicator(self.floor);

                    match self.behaviour {
                        ElevatorBehaviour::Moving => {
                            // If the elevator is moving, check if it has reached the next order. If not: keep moving.
                            if self.floor == self.nxt_order.floor
                            {
                                self.direction = Direction::Stop;
                                self.elevator.motor_direction(e::DIRN_STOP);
                                self.behaviour = ElevatorBehaviour::DoorOpen;
                                self.elevator.door_light(true);
                                self.start_door_timer(Duration::from_secs(3));                
                            }
                        },
                        _ => {},    // Hvis heisen ikke er i bevegelse, gjør ingenting
                    }
                }

                // Receive call buttons from elevator
                recv(self.channels.call_button_rx) -> msg => {
                    let call_button = msg.unwrap();
                    let new_call = tcp::CallButton { floor: call_button.floor, call: call_button.call };

                    println!("[SLAVE]\t\tReceived call button message: {:#?}", new_call);

                    // Send new order to master
                    match self.send_new_order(new_call) {
                        Ok(_)   => {},
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
                        println!("[SLAVE]\t\tObstruction detected. Timer reset.");
                    }
                    else {
                        println!("[SLAVE]\t\tTimer expired. Door closing.");
                        self.elevator.door_light(false);
                        self.behaviour = ElevatorBehaviour::Idle;
                        self.send_order_complete();
                    }
                }

                // Receive incoming message from master
                recv(self.channels.master_message_rx) -> msg => {
                    let message = msg.unwrap();
                    match message {
                        tcp::Message::NewOrder(callbutton) => {
                            // TEST if this is right!
                            if self.behaviour == ElevatorBehaviour::Idle {
                                self.nxt_order = callbutton.clone();
                                println!("[SLAVE]\t\tReceived new order: {:#?}", callbutton);
                                if self.floor == self.nxt_order.floor {
                                    self.behaviour = ElevatorBehaviour::DoorOpen;
                                    self.elevator.door_light(true);
                                    self.start_door_timer(Duration::from_secs(3));
                                }
                                else {
                                    self.start_moving();
                                }
                            }
                            else {
                                println!("[SLAVE]\t\tReceived new order, but elevator is not idle");
                            }
                        },
                        tcp::Message::LightMatrix(matrix) => {
                            self.light_matrix = matrix;
                            //println!("[SLAVE]\t\tReceived light matrix");
                            self.sync_lights();
                        },
                        tcp::Message::Error(_) => { println!("[SLAVE]\t\tReceived error message from master"); },
                        _ => {},   // Do nothing for OrderComplete messages and other messages
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
