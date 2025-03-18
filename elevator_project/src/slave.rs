use driver_rust::elevio::elev as e;
use crossbeam_channel as cbc;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::TcpStream;
use std::thread::{sleep, spawn};
use std::time::Duration;
use crate::config::Config;
use crate::inputs;
use crate::tcp;

const NUMBER_OF_FLOORS: u8 = 4;

// struct for orders in local operation mode
#[derive(Debug, Clone, Copy)]
pub struct LocalOrder {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    OutOfOrder,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Down = -1,
    Stop = 0,
    Up = 1,
}

#[derive(Debug)]
pub struct Slave {
    pub config      : Config,
    pub elevator    : e::Elevator,
    nxt_order       : tcp::CallButton,
    floor           : u8,
    obstruction     : bool,
    direction       : Direction,
    behaviour       : ElevatorBehaviour,
    channels        : inputs::SlaveChannels,
    master_channels : Option<(cbc::Sender<tcp::Message>, cbc::Receiver<tcp::Message>)>,     // If none, the elevator is in local mode
    door_timer      : (cbc::Sender<bool>, cbc::Receiver<bool>),
    light_matrix    : Vec<[bool; 3]>,                                                       // Hall_UP, Hall_DOWN, CAB_CALL for each floor

    // parameter for local operation mode:
    local_orders     : [LocalOrder; NUMBER_OF_FLOORS as usize],
}

impl Slave {
    pub fn init(slave_addr: String, config: &Config) -> Slave {
        let conf: Config = config.clone();
        let elev: e::Elevator = e::Elevator::init(&slave_addr, config.number_of_floors)
        .expect("[SLAVE]\t\tFailed to initialize elevator");
    
        let chs: inputs::SlaveChannels = inputs::spawn_threads_for_slave_inputs(
            &elev,
            conf.input_poll_rate_ms.clone(),
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
            master_channels: None,
            door_timer: cbc::unbounded::<bool>(),
            light_matrix: vec![[false; 3]; config.number_of_floors as usize],

            // parameter for local operation mode:
            local_orders        : [LocalOrder{
                                                hall_down   : false,
                                                hall_up     : false,
                                                cab_call    : false,    
                                            }; NUMBER_OF_FLOORS as usize],
        };


        
        // Turns all lights off
        slave.sync_lights_normal();
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

        slave.try_connect_to_new_master();

        if slave.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Starting in local operation mode.");
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
                    //send cab cue to master Hær kan vi ta med hall orders og!
                    let mut call_buttons_to_send = Vec::new();
                    for (floor, order) in self.local_orders.iter().enumerate() {
                        if order.cab_call { //eller skal vi ta med alt?
                            call_buttons_to_send.push(tcp::CallButton { floor: floor as u8, call: 2 });
                        }
                    }
                    
                    for call_button in call_buttons_to_send {
                        print!("Sending cab call to master: {:#?}", call_button);
                        self.send_new_order(call_button);
                    }
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
                .call_button_light(floor, e::HALL_UP, light_array[0]);
            self.elevator
                .call_button_light(floor, e::HALL_DOWN, light_array[1]);
            self.elevator
                .call_button_light(floor, e::CAB, light_array[2]);
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

        // append order to local_orders. Does not affect the elevator in normal operation mode
        match callbutton.call {
            0 => self.local_orders[callbutton.floor as usize].hall_up   = true,
            1 => self.local_orders[callbutton.floor as usize].hall_down = true,
            2 => self.local_orders[callbutton.floor as usize].cab_call  = true,
            _ => panic!("Mottok ukjent knappetype"),
        }

        if self.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match self.master_channels.as_mut().unwrap().0.send(message) {
            Ok(_) => {},
            Err(e) => {
                println!("[SLAVE]\t\tFailed to send order: {}", e);
                self.master_channels = None;
            }
        }
    }

    fn send_order_complete(&mut self) {
        let message = tcp::Message::OrderComplete(self.nxt_order);

        // remove order from local_orders list
        self.clear_at_current_floor(); 
        
        if self.master_channels.is_none() {
            println!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match self.master_channels.as_mut().unwrap().0.send(message) {
            Ok(_) => {},
            Err(e) => println!("[SLAVE]\t\tFailed to send order complete: {}", e),
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
        if  self.behaviour == ElevatorBehaviour::DoorOpen || 
            self.behaviour == ElevatorBehaviour::OutOfOrder
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

            /**************local operation mode***************/
            if self.master_channels.is_none() {

                cbc::select! {
                    recv(self.channels.call_button_rx) -> msg => {
                        let call_button = msg.unwrap();
                        println!("[SLAVE]\t\tReceived call button message: {:#?}", call_button);
            
                        // Update local orders
                        match call_button.call {
                            0 => self.local_orders[call_button.floor as usize].hall_up = true,
                            1 => self.local_orders[call_button.floor as usize].hall_down = true,
                            2 => self.local_orders[call_button.floor as usize].cab_call = true,
                            _ => panic!("[SLAVE]\t\tReceived unknown call button type"),
                        }
            
                        self.sync_lights_local();
                        
                        match self.behaviour {
                            ElevatorBehaviour::Idle => {
                                self.start_moving_local();
                            },
                            _ => {},
                        }
                    }
                    
                    // Receive floor sensor message
                    recv(self.channels.floor_sensor_rx) -> msg => {
                        let floor_sensor = msg.unwrap();
                        println!("[SLAVE]\t\tReceived floor sensor message: {:#?}", floor_sensor);
                        self.floor = floor_sensor;
            
                        match self.behaviour {
                            ElevatorBehaviour::Moving => { 
                                self.floor = floor_sensor;
                                self.elevator.floor_indicator(self.floor as u8);
                                if self.should_stop() {
                                    println!("[SLAVE]\t\tStopping at floor {:?}", self.floor);
                                    self.behaviour = ElevatorBehaviour::DoorOpen;
                                    self.elevator.door_light(true);
                                    self.clear_at_current_floor();
                                    self.sync_lights_local();
                                    self.elevator.motor_direction(e::DIRN_STOP);
            
                                    self.start_door_timer(Duration::from_secs(3));    // starting doortimer
                                }
                            },
                            _ => {},
                        }
                    }
            
                    // Receive stop button message
                    recv(self.channels.stop_button_rx) -> msg => {
                        let stop_button = msg.unwrap();
                        println!("[SLAVE]\t\tStop button: {:#?}", stop_button);
                        self.elevator.motor_direction(e::DIRN_STOP);
                        self.behaviour = ElevatorBehaviour::OutOfOrder; 
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
                            self.start_moving_local();
                        }
                    }
                    default(Duration::from_millis(self.config.input_poll_rate_ms)) =>  self.try_connect_to_new_master(),
                }// cbc::select!
            }// if master_channels.is_none


            /**************normal operation***************/
            else {
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
                                println!("[SLAVE]\t\tMoving. Floor: {:?}, next order {}", self.floor, self.nxt_order.floor);
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
                        self.send_new_order(new_call);
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
                    recv(self.master_channels.clone().unwrap().1) -> msg => {
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
                            tcp::Message::Error(_) => { 
                                println!("[SLAVE]\t\tReceived error message from master"); 
                                println!("[SLAVE]\t\tStarting in local operating mode");
                                self.master_channels = None;
                                if self.behaviour == ElevatorBehaviour::Idle{ //fiksa sånn at du ikkje må trykke to ganga
                                    self.sync_lights_local();
                                    self.start_moving_local();
                                }
                
                            },
                            _ => {},   // Do nothing for OrderComplete messages and other messages
                        }
                    }
                    default(Duration::from_millis(self.config.input_poll_rate_ms*100)) => {},
                }// cbc::select
            } // else
        } // loop
    }// slave_loop

    

    /************ functions for local operation mode **************/

    fn orders_above(&mut self) -> bool{
        for floor in (self.floor + 1) .. self.config.number_of_floors {
            if self.local_orders[floor as usize].hall_down || self.local_orders[floor as usize].hall_up || self.local_orders[floor as usize].cab_call {
                self.nxt_order = tcp::CallButton { floor: floor, call: 2};
                return true;
            }
        }
        return false;   
    }

    fn orders_below(&mut self) -> bool {
        for floor in 0 .. self.floor {
            if self.local_orders[floor as usize].hall_down || self.local_orders[floor as usize].hall_up || self.local_orders[floor as usize].cab_call {
                self.nxt_order = tcp::CallButton { floor: floor, call: 2};
                return true;
            }  
        }
        return false;
    }

    pub fn orders_here(&self) -> bool {
        return 
            self.local_orders[self.floor as usize].hall_down  || 
            self.local_orders[self.floor as usize].hall_up    || 
            self.local_orders[self.floor as usize].cab_call;
    }

    fn should_stop(&mut self) -> bool{
        match self.direction{
            Direction::Down => {
                self.local_orders[self.floor as usize].hall_down ||
                self.local_orders[self.floor as usize].cab_call  ||
                !self.orders_below()
            }
            Direction::Up => {
                self.local_orders[self.floor as usize].hall_up   ||
                self.local_orders[self.floor as usize].cab_call  ||
                !self.orders_above()
            }
            _=> true
        }
    }

    fn choose_direction(&mut self) -> (Direction, ElevatorBehaviour) {
        match self.direction {
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
        self.local_orders[self.floor as usize].cab_call    = false;
        self.local_orders[self.floor as usize].hall_down   = false;
        self.local_orders[self.floor as usize].hall_up     = false;
    }

    fn start_moving_local(&mut self) {
        let (diraction, behaviour) = self.choose_direction();
        self.nxt_order = tcp::CallButton { floor: 1, call: 0};
        self.behaviour = behaviour;
        
        if behaviour == ElevatorBehaviour::DoorOpen {
            println!("Stopped with door open at floor {:?}", self.floor);
            self.clear_at_current_floor(); //den opna ikkje døra når den fikk order i samme etasje so de va, so la til 2 linje som fiksa det
            self.sync_lights_local();
            self.elevator.door_light(true);
            self.start_door_timer(Duration::from_secs(3));
        }

        match diraction {
            Direction::Up   => {
                self.elevator.motor_direction(e::DIRN_UP);
                self.direction = Direction::Up;
            },
            Direction::Down => {
                self.elevator.motor_direction(e::DIRN_DOWN);
                self.direction = Direction::Down;
            },
            Direction::Stop => {
                self.elevator.motor_direction(e::DIRN_STOP);
                self.direction = Direction::Stop;
            },
        }
    }

    fn sync_lights_local(&self) {
        for (floor, order) in self.local_orders.iter().enumerate() {
            let floor = floor as u8;
            self.elevator.call_button_light(floor, e::HALL_UP,    order.hall_up);
            self.elevator.call_button_light(floor, e::HALL_DOWN,  order.hall_down);
            self.elevator.call_button_light(floor, e::CAB,        order.cab_call);
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
            self.master_channels,
            self.door_timer
        )
    }
}
