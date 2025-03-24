
use crossbeam_channel as cbc;
use driver_rust::elevio::elev::{self as e, HALL_DOWN, HALL_UP, CAB, DIRN_DOWN, DIRN_UP, DIRN_STOP};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::TcpStream;
use std::thread::{sleep, spawn};
use std::time::Duration;
use crate::config::{Config, NUMBER_OF_FLOORS};
use crate::inputs;
use crate::tcp;

// struct for orders in local operation mode
#[derive(Debug, Clone, Copy)]
pub struct LocalOrder {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    OutOfOrder,
}
impl ElevatorBehaviour{
    pub fn to_ascii_lowercase(self) -> &'static str{
        match self {
            ElevatorBehaviour::Idle => "idle",
            ElevatorBehaviour::Moving => "moving",
            ElevatorBehaviour::DoorOpen => "doorOpen",
            ElevatorBehaviour::OutOfOrder => "outOfOrder",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Down = -1,
    Stop = 0,
    Up = 1,
}
impl Direction {
    pub fn to_ascii_lowercase(self) -> &'static str{
        match self {
            Direction::Down => "down",
            Direction::Stop => "stop",
            Direction::Up => "up",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ElevatorState {
    pub behaviour           : ElevatorBehaviour,
    pub floor               : u8,
    pub direction           : Direction,
    pub cab_requests        : [bool; NUMBER_OF_FLOORS],
}
impl ElevatorState{
    pub fn init() -> ElevatorState {
        ElevatorState {
            behaviour       : ElevatorBehaviour::OutOfOrder,
            floor           : 0,
            direction       : Direction::Stop,
            cab_requests    : [false; NUMBER_OF_FLOORS]
        }
    }
}

impl Display for ElevatorState {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Behaviour:\t{:#?}\nFloor:\t{:#?}\nDirection:\t{:#?},\nCab Requests:\t{:#?}",
            self.behaviour,
            self.floor,
            self.direction,
            self.cab_requests
        )
    }
}

#[derive(Debug)]
pub struct Slave {
    pub config      : Config,
    pub elevator    : e::Elevator,
    pub state       : ElevatorState,
    obstruction     : bool,
    nxt_order       : tcp::CallButton,
    channels        : inputs::SlaveChannels,
    master_channels : Option<(cbc::Sender<tcp::Message>, cbc::Receiver<tcp::Message>)>,     // If none, the elevator is in local mode
    door_timer      : (cbc::Sender<bool>, cbc::Receiver<bool>),
    light_matrix    : [[bool; 3]; NUMBER_OF_FLOORS],                                       // Hall_UP, Hall_DOWN, CAB_CALL for each floor
}

impl Slave {
    pub fn init(slave_addr: String, config: &Config) -> Slave {
        let conf: Config = config.clone();
        let elev: e::Elevator = e::Elevator::init(&slave_addr, NUMBER_OF_FLOORS as u8)
        .expect("[SLAVE]\t\tFailed to initialize elevator");
    
        let chs: inputs::SlaveChannels = inputs::spawn_threads_for_slave_inputs(
            &elev,
            conf.input_poll_rate_ms.clone(),
        );

        let mut slave = Self {
            config: conf,
            elevator: elev,
            nxt_order           : tcp::CallButton { floor: 0, call: 0 },
            state               : ElevatorState::init(),
            obstruction         : false,
            channels            : chs,
            master_channels     : None,
            door_timer          : cbc::unbounded::<bool>(),
            light_matrix        : [[false; 3]; NUMBER_OF_FLOORS],
        };
        
        // Turns all lights off
        slave.sync_lights_normal();
        slave.elevator.door_light(false);

        // Initiate elevator position and lights to the nearest floor in downwards direction
        slave.state.behaviour = ElevatorBehaviour::Moving;
        slave.state.direction = Direction::Down;
        slave.elevator.motor_direction(DIRN_DOWN);
        loop {
            cbc::select! {
                recv(slave.channels.floor_sensor_rx) -> msg => {
                    let floor_sensor = msg.unwrap();
                    println!("Received floor sensor message: {:#?}", floor_sensor);
                    slave.state.floor = floor_sensor;
                    if slave.state.floor !=u8::MAX{
                        slave.elevator.motor_direction(DIRN_STOP);
                        slave.state.direction = Direction::Stop;
                        slave.state.behaviour = ElevatorBehaviour::Idle;
                        slave.elevator.floor_indicator(slave.state.floor as u8);
                        break;
                    }
                }
            }
        }

        slave.try_connect_to_new_master();

        if slave.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Starting in local operation mode.");
        }
        else{
            println!("[SLAVE]\t\tConnected to master. Starting in normal operation mode.");
            slave.send_state_update();
        }
        return slave;
    }

  
    fn try_connect_to_new_master(&mut self) {
        for ip_addr in &self.config.elevator_ip_list {
            let socket_addr = std::net::SocketAddrV4::new(*ip_addr, self.config.master_port);

            match TcpStream::connect(socket_addr) {
                Ok(stream) => {
                    println!("[SLAVE]\t\tConnected to master at {}:{}", ip_addr, self.config.master_port);
                    self.master_channels = Some(inputs::spawn_thread_for_master_connection(stream, self.config.input_poll_rate_ms));
                    //send status til master
                    //self.send_state_update(); trur ikkje vi trenge ditta siden det blir gjort inne i set behavior og
                    //Stop the elevator, and let the master decide what to do 
                    //ditta gjær at de blir et lite hakk, men e de innafor siden de e beire en at den kjøre utforbi
                    self.elevator.motor_direction(DIRN_STOP);
                    self.set_behaviour(ElevatorBehaviour::Idle);
                    return;
                },
                Err(_e) => {}   // Continue trying with the next IP address  
            }
        }
    }

    // Poll light information from dirver and update light_matrix
    fn sync_lights_normal(&self) {
        println!("Syncing lights");
        for (floor_index, light_array) in self.light_matrix.iter().enumerate() {
            let floor = floor_index as u8;
            self.elevator
                .call_button_light(floor, HALL_UP, light_array[0]);
            self.elevator
                .call_button_light(floor, HALL_DOWN, light_array[1]);
            self.elevator
                .call_button_light(floor, CAB, light_array[2]);
        }
    }

    // Spawn a new thread that will sleep for the given duration and then send a message to the door_timer channel when done. 
    fn start_door_timer(&self, duration: Duration) {
        let tx = self.door_timer.0.clone();
        spawn(move || {
            sleep(duration);
            let _ = tx.send(true).unwrap();
        });
    }

    fn send_new_order(&mut self, callbutton: tcp::CallButton) {
        let message = tcp::Message::NewOrder(callbutton.clone());

        if self.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match callbutton.call {
            0_u8..=1_u8 => {
                match   self.master_channels
                            .as_mut()
                            .unwrap()
                            .0
                            .send(message) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("[SLAVE]\t\tFailed to send order: {}", e);
                        self.master_channels = None;
                    }
                }
            },
            2 => {
                self.state.cab_requests[callbutton.floor as usize] = true;
                self.send_state_update();
            },
            _ => panic!("Mottok ukjent knappetype"),
        }
    }

    fn send_order_complete(&mut self) {
        self.state.cab_requests[self.state.floor as usize]   = false;
        self.send_state_update();

         if self.nxt_order.call != CAB{
            let message = tcp::Message::OrderComplete(self.nxt_order);
            
            if self.master_channels.is_none() {
                println!("[SLAVE]\t\tNo master found. Cannot send order.");
                return;
            }

            match self.master_channels.as_mut().unwrap().0.send(message) {
                Ok(_) => {println!("[SLAVE]\t\tSent order complite");},
                Err(e) => println!("[SLAVE]\t\tFailed to send order complete: {}", e),
            }
        }
    }

    fn send_stop_button(&mut self) {
        let message = tcp::Message::Error(tcp::ErrorState::EmergancyStop);

        if self.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match self.master_channels.as_mut().unwrap().0.send(message) {
            Ok(_) => {},
            Err(e) => println!("[SLAVE]\t\tFailed to send stop button: {}", e),
        }
    }

    fn send_idle(&mut self) {
        let message = tcp::Message::Idle;
        if self.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match self.master_channels.as_mut().unwrap().0.send(message) {
            Ok(_) => {},
            Err(e) => println!("[SLAVE]\t\tFailed to send idle: {}", e),
        }
    }

    // Choose direction based on next order and start moving. 
    fn start_moving_normal(&mut self) {
        if  self.state.behaviour == ElevatorBehaviour::DoorOpen || 
            self.state.behaviour == ElevatorBehaviour::OutOfOrder
        {
            // Do nothing if the elevator is out of order or the door is open
            return;
        }

        if self.state.floor > self.nxt_order.floor {
            self.state.direction = Direction::Down;
            self.set_behaviour(ElevatorBehaviour::Moving);
        } else {
            self.state.direction = Direction::Up;
            self.set_behaviour(ElevatorBehaviour::Moving);
        }
        match self.state.direction {
            Direction::Stop => self.elevator.motor_direction(DIRN_STOP),
            Direction::Down => self.elevator.motor_direction(DIRN_DOWN),
            Direction::Up   => self.elevator.motor_direction(DIRN_UP),
        }
    }


    pub fn send_state_update(&mut self) {
        let message = tcp::Message::StateUpdate(self.state.clone());
        if self.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match self.master_channels.as_mut().unwrap().0.send(message) {
            Ok(_) => {},
            Err(e) => println!("[SLAVE]\t\tFailed to send status update: {}", e),
        }

    }


    fn set_behaviour(&mut self, new_behaviour: ElevatorBehaviour) {
        if new_behaviour != self.state.behaviour {
            if new_behaviour != ElevatorBehaviour::OutOfOrder {
                self.state.behaviour = new_behaviour;
                self.send_state_update();
            }
            self.state.behaviour = new_behaviour;
        }
    }


    // State machine for the slave elevator
    pub fn slave_loop(&mut self) {
        loop {

            /**************local operation mode***************/
            if self.master_channels.is_none() {
                cbc::select! {
                    // Receive call button message from elevator
                    recv(self.channels.call_button_rx) -> msg => {
                        let call_button = msg.unwrap();
                        println!("[SLAVE]\t\tReceived call button message: {:#?}", call_button);
            
                        // Update local cab requests
                        if call_button.call == 2 {
                            self.state.cab_requests[call_button.floor as usize] = true;
                        }
            
                        self.sync_lights_local();
                        
                        match self.state.behaviour {
                            ElevatorBehaviour::Idle => {
                                self.start_moving_local();
                            },
                            _ => {},
                        }
                    }
                    
                    // Receive floor sensor message from elevator
                    recv(self.channels.floor_sensor_rx) -> msg => {
                        let floor_sensor = msg.unwrap();
                        println!("[SLAVE]\t\tReceived floor sensor message: {:#?}", floor_sensor);
                        self.state.floor = floor_sensor;
            
                        match self.state.behaviour {
                            ElevatorBehaviour::Moving => { 
                                self.state.floor = floor_sensor;
                                self.elevator.floor_indicator(self.state.floor as u8);
                                if self.should_stop() {
                                    println!("[SLAVE]\t\tStopping at floor {:?}", self.state.floor);
                                    self.set_behaviour(ElevatorBehaviour::DoorOpen);
                                    self.elevator.door_light(true);
                                    self.clear_at_current_floor();
                                    self.sync_lights_local();
                                    self.elevator.motor_direction(DIRN_STOP);
            
                                    self.start_door_timer(Duration::from_secs(3));    // starting doortimer
                                }
                            },
                            _ => {},
                        }
                    }
            
                    // Receive stop button message from elevator
                    recv(self.channels.stop_button_rx) -> msg => {
                        let stop_button = msg.unwrap();
                        println!("[SLAVE]\t\tStop button: {:#?}", stop_button);
                        self.elevator.motor_direction(DIRN_STOP);
                        self.set_behaviour(ElevatorBehaviour::OutOfOrder);
                    }
            
                    recv(self.channels.obstruction_rx) -> msg => {
                        let obstr = msg.unwrap();
                        self.obstruction = obstr;
            
                        println!("[SLAVE]\t\tObstruction: {:#?}", obstr);
                    }
            
                    // Receive timer message
                    recv(self.door_timer.1) -> _msg => {
                        if self.obstruction {
                            //println!("Obstruction detected. Timer reset.");
                            self.start_door_timer(Duration::from_secs(3));
                        }
                        else {
                            println!("[SLAVE]\t\tTimer expired. Door closing.");
                            self.elevator.door_light(false);
                            self.set_behaviour(ElevatorBehaviour::Idle);
                            self.start_moving_local();
                        }
                    }
                    default(Duration::from_millis(self.config.input_poll_rate_ms)) =>  self.try_connect_to_new_master(),
                }// cbc::select!
            }// if master_channels.is_none


            /**************normal operation***************/
            else {
                cbc::select! {
                    // Receive floor sensor from elevator
                    recv(self.channels.floor_sensor_rx) -> msg => {
                        let floor_sensor = msg.unwrap();
                        self.state.floor = floor_sensor;
                        self.send_state_update(); // jobba med å få til å ta ordre på veien so la til dinna, men går ikkje endå
                        self.elevator.floor_indicator(self.state.floor);
                        if self.state.floor == self.nxt_order.floor{
                            self.state.direction = Direction::Stop;
                            self.elevator.motor_direction(DIRN_STOP);
                            self.set_behaviour(ElevatorBehaviour::DoorOpen);
                            self.elevator.door_light(true);
                            self.start_door_timer(Duration::from_secs(3)); 
                            self.send_order_complete();
                        }
                    }

                    // Receive call buttons from elevator
                    recv(self.channels.call_button_rx) -> msg => {
                        let call_button = msg.unwrap();
                        let new_call = tcp::CallButton { floor: call_button.floor, call: call_button.call };
                        println!("[SLAVE]\t\tReceived call button message: {:#?}", new_call);
                        self.send_new_order(new_call);
                    }

                    // Receive stop button from elevator
                    recv(self.channels.stop_button_rx) -> msg => {
                        let stop_button = msg.unwrap();
                        println!("[SLAVE]\t\tStop button: {:#?}", stop_button);
                        self.elevator.motor_direction(DIRN_STOP);
                        self.set_behaviour(ElevatorBehaviour::OutOfOrder);
                        self.send_stop_button();
                    }

                    // Receive obstruction from elevator. If obstruction is detected, send a message to master to reassign hall orders.
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
                            self.send_state_update(); //sånn at lysmatrisa blir oppdatert skjølv om heisen e stuck
                        }
                        else {
                            println!("[SLAVE]\t\tTimer expired. Door closing.");
                            self.elevator.door_light(false);
                            self.set_behaviour(ElevatorBehaviour::Idle);
                          //self.send_order_complete();
                        }
                    }

                    // Receive incoming message from master
                    recv(self.master_channels.clone().unwrap().1) -> msg => {
                        let message = msg.unwrap();
                        match message {
                            tcp::Message::NewOrder(callbutton) => {
                                if self.state.behaviour == ElevatorBehaviour::Idle { //trur den må fjernast får å kunne ta ordre på veien, men fekk ikkje det til
                                    self.nxt_order = callbutton.clone();
                                    //println!("[SLAVE]\t\tReceived new order from master: {:#?}", callbutton);
                                    println!("[SLAVE]\t floor: {:#?}, nxt_order: {:#?}", self.state.floor, self.nxt_order.floor);
                                    if self.state.floor == self.nxt_order.floor {
                                        self.set_behaviour(ElevatorBehaviour::DoorOpen);
                                        self.elevator.door_light(true);
                                        self.start_door_timer(Duration::from_secs(3));
                                        self.send_order_complete();
                                    }
                                    else {
                                        self.start_moving_normal();
                                    }
                                }
                                else {
                                   println!("[SLAVE]\t\tReceived new order, but elevator is not idle");
                                }
                            },
                            tcp::Message::LightMatrix(matrix) => {
                                self.light_matrix = matrix;
                                //println!("[SLAVE]\t\tReceived light matrix");
                                self.sync_lights_normal();
                            },
                            // Receive state update from master. Used to syncronize the state of the elevator when reconnecting to the master.
                            tcp::Message::StateUpdate(state) => {     
                                for i in 0..NUMBER_OF_FLOORS {
                                    if state.cab_requests[i] {
                                        self.state.cab_requests[i] = state.cab_requests[i]; //sånn at den behelde dei ordrane den hadde i localt modus
                                    }
                                }
                                self.send_state_update();
                                //println!("[SLAVE]\t\tReceived state update");
                            },
                            tcp::Message::Error(_) => { 
                                println!("[SLAVE]\t\tReceived error message from master"); 
                                println!("[SLAVE]\t\tStarting in local operating mode");
                                self.master_channels = None;

                                // turn off all hall lights since we are in local mode and no longer take hall orders
                                for i in 0..NUMBER_OF_FLOORS {
                                    self.elevator.call_button_light(i as u8, HALL_UP, false);
                                    self.elevator.call_button_light(i as u8, HALL_DOWN, false);
                                }
                                if self.state.behaviour == ElevatorBehaviour::Idle{ 
                                    self.sync_lights_local();
                                    self.start_moving_local();
                                }
                
                            },
                            _ => {},   // Do nothing for OrderComplete messages and other messages
                        }
                    }
                    default(Duration::from_millis(self.config.input_poll_rate_ms*100)) => {
                        if self.state.behaviour == ElevatorBehaviour::Idle {
                            self.send_idle();
                        }
                    },
                }// cbc::select
            } // else
        } // loop
    }// slave_loop

    

    /************ functions for local operation mode **************/

    fn orders_above(&mut self) -> bool{
        for floor in (self.state.floor + 1) .. NUMBER_OF_FLOORS as u8 {
            if self.state.cab_requests[floor as usize] {
                self.nxt_order = tcp::CallButton { floor: floor, call: CAB};
                return true;
            }
        }
        return false;   
    }

    fn orders_below(&mut self) -> bool {
        for floor in 0 .. self.state.floor {
            if self.state.cab_requests[floor as usize] {
                self.nxt_order = tcp::CallButton { floor: floor, call: CAB};
                return true;
            }  
        }
        return false;
    }

    pub fn orders_here(&self) -> bool {
        return self.state.cab_requests[self.state.floor as usize];
    }

    fn should_stop(&mut self) -> bool{
        match self.state.direction{
            Direction::Down => {
                self.state.cab_requests[self.state.floor as usize]  || !self.orders_below()
            }
            Direction::Up => {
                self.state.cab_requests[self.state.floor as usize]  || !self.orders_above()
            }
            _=> true
        }
    }

    fn choose_direction(&mut self) -> (Direction, ElevatorBehaviour) {
        match self.state.direction {
            Direction::Up => { return
                if      self.orders_above() { ( Direction::Up,   ElevatorBehaviour::Moving ) }
                else if self.orders_here()  { ( Direction::Down, ElevatorBehaviour::DoorOpen ) }
                else if self.orders_below() { ( Direction::Down, ElevatorBehaviour::Moving ) }
                else                        { ( Direction::Stop, ElevatorBehaviour::Idle ) }
            }

            Direction::Down => { return 
                if      self.orders_below() { ( Direction::Down, ElevatorBehaviour::Moving ) }
                else if self.orders_here()  { ( Direction::Up,   ElevatorBehaviour::DoorOpen ) }
                else if self.orders_above() { ( Direction::Up,   ElevatorBehaviour::Moving ) }
                else                        { ( Direction::Stop, ElevatorBehaviour::Idle ) }
            }

            Direction::Stop => { return 
                if      self.orders_here()  { ( Direction::Stop, ElevatorBehaviour::DoorOpen ) }
                else if self.orders_above() { ( Direction::Up,   ElevatorBehaviour::Moving ) }
                else if self.orders_below() { ( Direction::Down, ElevatorBehaviour::Moving ) }
                else                        { ( Direction::Stop, ElevatorBehaviour::Idle ) }
            }
        }
    }

    fn clear_at_current_floor(&mut self) {
        self.state.cab_requests[self.state.floor as usize]   = false;
    }

    fn start_moving_local(&mut self) {
        let (diraction, behaviour) = self.choose_direction();
        self.nxt_order = tcp::CallButton { floor: 1, call: CAB};
        self.state.behaviour = behaviour;
        
        if behaviour == ElevatorBehaviour::DoorOpen {
            println!("Stopped with door open at floor {:?}", self.state.floor);
            self.clear_at_current_floor(); //den opna ikkje døra når den fikk order i samme etasje so de va, so la til 2 linje som fiksa det
            self.sync_lights_local();
            self.elevator.door_light(true);
            self.start_door_timer(Duration::from_secs(3));
        }

        match diraction {
            Direction::Up   => {
                self.elevator.motor_direction(DIRN_UP);
                self.state.direction = Direction::Up;
            },
            Direction::Down => {
                self.elevator.motor_direction(DIRN_DOWN);
                self.state.direction = Direction::Down;
            },
            Direction::Stop => {
                self.elevator.motor_direction(DIRN_STOP);
                self.state.direction = Direction::Stop;
            },
        }
    }

    
    fn sync_lights_local(&self) {
        for (floor, order) in self.state.cab_requests.iter().enumerate() {
            self.elevator.call_button_light(floor as u8, e::CAB,        *order);
        }
    }
}

impl Display for Slave {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "\tElevator:\t{:#?}\n\
            \tState:\t{:#?}\n\
            \tNxt_order:\t{:#?}\n\
            \tObstruction:\t{:#?}\n\
            \tChannels:\t{:#?}\n\
            \tMaster_socket:\t{:#?}\n\
            \tDoor_timer:\t{:#?}",
            self.elevator,
            self.state,
            self.nxt_order,
            self.obstruction,
            self.channels,
            self.master_channels,
            self.door_timer
        )
    }
}
