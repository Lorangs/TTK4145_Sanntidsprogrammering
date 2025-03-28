use crate::config::{Config, NUMBER_OF_FLOORS};
use crate::io_datastructures::{
    CallButton, Direction, ElevatorBehaviour, ElevatorState, ErrorState, Message,
};
use crate::slave_inputs;
use crate::heartbeat;
use crossbeam_channel as cbc;
use debug_print::debug_println as dprintln;
use driver_rust::elevio::elev::{
    self as e, CAB, DIRN_DOWN, DIRN_STOP, DIRN_UP, HALL_DOWN, HALL_UP,
};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use network_rust::udpnet;

#[derive(Debug)]
pub struct Slave {
    pub config: Config,
    pub elevator: e::Elevator,
    pub state: ElevatorState,
    obstruction: bool,
    stop_button: bool,
    next_order: CallButton,
    channels: slave_inputs::SlaveChannels,
    master_channels: Option<(cbc::Sender<Message>, cbc::Receiver<Message>)>, // If None the elevator is in local mode
    door_timer: (cbc::Sender<bool>, cbc::Receiver<bool>),
    motor_timeout: (cbc::Sender<bool>, cbc::Receiver<bool>),
    heartbeat_rx: cbc::Receiver<udpnet::peers::PeerUpdate>,
    timestamp_prev_floor: Instant,
    light_matrix: [[bool; 2]; NUMBER_OF_FLOORS], // [Hall_UP, Hall_DOWN] for each floor
}

impl Slave {
    /// Initialize a new slave unit
    pub fn init(config: &Config, slave_num: String) -> Slave {
        let conf: Config = config.clone();
        let elev: e::Elevator = e::Elevator::init(
            ("localhost:".to_string() + config.elevator_port.to_string().as_str()).as_str(),
            NUMBER_OF_FLOORS as u8,
        )
        .expect("[SLAVE]\t\tFailed to initialize elevator");

        let chs: slave_inputs::SlaveChannels =
        slave_inputs::spawn_threads_for_slave_inputs(&elev, conf.input_poll_rate_ms);
        
        let (heart_update_tx, heart_update_rx) = cbc::unbounded::<udpnet::peers::PeerUpdate>();
        heartbeat::recieve_online_status(heart_update_tx, config.heartbeat_port);
        heartbeat::send_alive(slave_num,config.heartbeat_port);   
        
        let mut slave = Self {
            config: conf,
            elevator: elev,
            next_order: CallButton { floor: 0, call: 0 }, // Need to be initialized, but not used until a new order is received
            state: ElevatorState::init(),
            obstruction: false,
            stop_button: false,
            channels: chs,
            master_channels: None,
            door_timer: cbc::unbounded::<bool>(),
            motor_timeout: cbc::unbounded::<bool>(),
            heartbeat_rx: heart_update_rx,
            timestamp_prev_floor: Instant::now(),
            light_matrix: [[false; 2]; NUMBER_OF_FLOORS],
        };
        
        // Turns all lights off
        slave.sync_hall_lights();
        slave.sync_cab_lights();
        slave.elevator.door_light(false);
        
        // Initiate elevator position and lights to the nearest floor in downwards direction
        slave.state.behaviour = ElevatorBehaviour::Moving;
        slave.state.direction = Direction::Down;
        slave.elevator.motor_direction(DIRN_DOWN);
        loop {
            cbc::select! {
                recv(slave.channels.floor_sensor_rx) -> msg => {
                    let floor_sensor = msg.unwrap();
                    dprintln!("[SLAVE]\t\tReceived floor sensor message: {:#?}", floor_sensor);
                    slave.state.floor = floor_sensor;
                    if slave.state.floor !=u8::MAX
                    {
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
            dprintln!("[SLAVE]\t\tNo master found. Starting in local operation mode.");
        } else {
            dprintln!("[SLAVE]\t\tConnected to master. Starting in normal operation mode.");
            slave.send_state_update();
        }
        

        slave
    }

    /// Iter through the list of IP addresses and try to connect to a master at each address.
    /// Return when a connection is established or none is found.
    fn try_connect_to_new_master(&mut self) {
        for ip_addr in &self.config.elevator_ip_list {
            let socket_addr =
                std::net::SocketAddr::new(std::net::IpAddr::V4(*ip_addr), self.config.master_port);

            match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(self.config.tcp_timeout_ms)) {
                Ok(stream) => {
                    dprintln!(
                        "[SLAVE]\t\tConnected to master at {}:{}",
                        ip_addr,
                        self.config.master_port
                    );
                    self.master_channels = Some(slave_inputs::spawn_thread_for_master_connection(
                        stream,
                        self.config.input_poll_rate_ms,
                    ));
                    //Stop the elevator, and let the master decide what to do
                    self.elevator.motor_direction(DIRN_STOP);
                    self.set_behaviour(ElevatorBehaviour::Idle);
                    return;
                }
                Err(_) => {} // Continue trying with the next IP address
            }
        }
    }

    fn sync_hall_lights(&self) {
        dprintln!("[SLAVE]\t\tSyncing hall lights");
        for (floor, light_array) in self.light_matrix.iter().enumerate() {
            self.elevator
                .call_button_light(floor as u8, HALL_UP, light_array[0]);
            self.elevator
                .call_button_light(floor as u8, HALL_DOWN, light_array[1]);
        }
    }

    fn sync_cab_lights(&self) {
        dprintln!("[SLAVE]\t\tSyncing cab lights");
        for (floor, order) in self.state.cab_requests.iter().enumerate() {
            self.elevator.call_button_light(floor as u8, e::CAB, *order);
        }
    }

    fn send_new_order(&mut self, callbutton: CallButton) {
        let message = Message::NewOrder(callbutton);

        if self.master_channels.is_none() {
            dprintln!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match callbutton.call {
            HALL_DOWN | HALL_UP => match self.master_channels.as_mut().unwrap().0.send(message) {
                Ok(_) => {}
                Err(_e) => {
                    dprintln!("[SLAVE]\t\tFailed to send order: {}", _e);
                    self.master_channels = None;
                }
            },
            CAB => {
                self.state.cab_requests[callbutton.floor as usize] = true;
                self.send_state_update();
                self.sync_cab_lights();
            }
            _ => {
                dprintln!(
                    "[SLAVE]\t\tInvalid call button. Hardware failiure: {}",
                    callbutton.call
                )
            }
        }
    }

    fn send_order_complete(&mut self) {
        self.state.cab_requests[self.state.floor as usize] = false;
        self.send_state_update();
        self.sync_cab_lights();

        if self.next_order.call != CAB {
            let message = Message::OrderComplete(self.next_order);

            if self.master_channels.is_none() {
                dprintln!("[SLAVE]\t\tNo master found. Cannot send order.");
                return;
            }

            match self.master_channels.as_mut().unwrap().0.send(message) {
                Ok(_) => {
                    dprintln!("[SLAVE]\t\tSent order complite");
                }
                Err(_e) => {
                    dprintln!("[SLAVE]\t\tFailed to send order complete: {}", _e)
                }
            }
        }
    }

    fn send_stop_button(&mut self) {
        let message = Message::Error(ErrorState::EmergancyStop);

        if self.master_channels.is_none() {
            dprintln!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        match self.master_channels.as_mut().unwrap().0.send(message) {
            Ok(_) => {}
            Err(_e) => {
                dprintln!("[SLAVE]\t\tFailed to send stop button: {}", _e)
            }
        }
    }

    /// Choose direction based on next order from master and start moving
    fn start_moving_normal(&mut self) {
        if self.state.behaviour == ElevatorBehaviour::DoorOpen
            || self.state.behaviour == ElevatorBehaviour::OutOfOrder
        {
            return; // Do nothing if the elevator is out of order or the door is open
        }

        slave_inputs::start_timer(self.motor_timeout.0.clone(), self.config.est_moving_time_s);
        self.timestamp_prev_floor = Instant::now();

        if self.state.floor > self.next_order.floor {
            self.state.direction = Direction::Down;
            self.set_behaviour(ElevatorBehaviour::Moving);
        } else {
            self.state.direction = Direction::Up;
            self.set_behaviour(ElevatorBehaviour::Moving);
        }
        match self.state.direction {
            Direction::Stop => self.elevator.motor_direction(DIRN_STOP),
            Direction::Down => self.elevator.motor_direction(DIRN_DOWN),
            Direction::Up => self.elevator.motor_direction(DIRN_UP),
        }
    }

    pub fn send_state_update(&mut self) {
        if self.master_channels.is_none() {
            dprintln!("[SLAVE]\t\tNo master found. Cannot send order.");
            return;
        }

        let message = Message::StateUpdate(self.state);
        match self.master_channels.as_mut().unwrap().0.send(message) {
            Ok(_) => {}
            Err(_e) => {
                dprintln!("[SLAVE]\t\tFailed to send status update: {}", _e)
            }
        }
    }

    /// Set the behaviour of the elevator. If the behaviour is changed, send a state update to the master
    fn set_behaviour(&mut self, new_behaviour: ElevatorBehaviour) {
        if new_behaviour != self.state.behaviour {
            self.state.behaviour = new_behaviour;
            if new_behaviour != ElevatorBehaviour::OutOfOrder {
                self.send_state_update();
            }
        }
    }

    /// State machine for the slave unit
    pub fn slave_loop(&mut self) {
        loop {
            /************** normal operation ***************/
            if self.master_channels.is_some() {
                cbc::select! {

                    // Receive floor sensor from elevator
                    recv(self.channels.floor_sensor_rx) -> msg => {
                        let floor_sensor = msg.unwrap();
                        self.state.floor = floor_sensor;

                        slave_inputs::start_timer(self.motor_timeout.0.clone(), self.config.est_moving_time_s);
                        self.timestamp_prev_floor = std::time::Instant::now();

                        self.elevator.floor_indicator(self.state.floor);

                        if self.state.floor == self.next_order.floor{
                            self.state.direction = Direction::Stop;
                            self.elevator.motor_direction(DIRN_STOP);
                            self.set_behaviour(ElevatorBehaviour::DoorOpen);
                            self.elevator.door_light(true);
                            self.send_order_complete();
                            slave_inputs::start_timer(self.door_timer.0.clone(), self.config.door_open_duration_s);
                        }
                    }

                    // Receive call buttons from elevator
                    recv(self.channels.call_button_rx) -> msg => {
                        let call_button = msg.unwrap();
                        let new_call = CallButton { floor: call_button.floor, call: call_button.call };
                        dprintln!("[SLAVE]\t\tReceived call button message: {:#?}", new_call);
                        self.send_new_order(new_call);
                    }

                    // Receive stop button from elevator
                    recv(self.channels.stop_button_rx) -> msg => {
                        self.stop_button = msg.unwrap();
                        dprintln!("[SLAVE]\t\tStop button:\t{:#?}", self.stop_button);
                        if self.stop_button {
                            self.elevator.motor_direction(DIRN_STOP);
                            self.set_behaviour(ElevatorBehaviour::OutOfOrder);
                            self.send_stop_button();
                            return;
                        }
                        else {
                            self.set_behaviour(ElevatorBehaviour::Idle);
                        }
                    }

                    // Receive obstruction from elevator
                    recv(self.channels.obstruction_rx) -> msg => {
                        let obstr = msg.unwrap();
                        self.obstruction = obstr;
                        dprintln!("[SLAVE]\t\tObstruction:\t{:#?}", obstr);
                    }

                    // Receive door timer expiration from door_timer
                    recv(self.door_timer.1) -> _msg => {
                        if self.obstruction {
                            slave_inputs::start_timer(self.door_timer.0.clone(), self.config.door_open_duration_s);
                            dprintln!("[SLAVE]\t\tObstruction detected. Door timer reset.");
                            self.set_behaviour(ElevatorBehaviour::OutOfOrder);
                            self.send_state_update();
                        }
                        else {
                            dprintln!("[SLAVE]\t\tDoor timer expired. Closing door.");
                            self.elevator.door_light(false);
                            self.set_behaviour(ElevatorBehaviour::Idle);
                        }
                    }

                    // Receive motor timeout if the elevator has not reached a floor within the estimated moving time set in config file.
                    recv(self.motor_timeout.1) -> _msg => {
                        if      self.timestamp_prev_floor + Duration::from_secs(self.config.est_moving_time_s) < std::time::Instant::now()
                            &&  self.state.behaviour == ElevatorBehaviour::Moving
                            &&  !self.stop_button
                        {
                            dprintln!("[SLAVE]\t\tMotor timeout. Out of order.");
                            self.set_behaviour(ElevatorBehaviour::OutOfOrder);
                            self.send_state_update();
                        }
                    }

                    //Detects a master disconnection
                    recv(self.heartbeat_rx)-> msg => {
                        for ip in msg.unwrap().lost{
                            if ip.trim()=="Master".to_string(){
                                dprintln!("[SLAVE]\t\tNo heartbeat from master");
                                dprintln!("[SLAVE]\t\tStarting in local operating mode");
                                self.master_channels = None;

                                // Turn off all hall lights since we are in local mode and no longer take hall orders
                                for i in 0..NUMBER_OF_FLOORS {
                                    self.elevator.call_button_light(i as u8, HALL_UP, false);
                                    self.elevator.call_button_light(i as u8, HALL_DOWN, false);
                                }
                                if self.state.behaviour == ElevatorBehaviour::Idle{
                                    self.start_moving_local();
                                }                           
                            }
                        }
                    }

                    // Receive incoming message from master
                    recv(self.master_channels.clone().unwrap().1) -> msg => {
                        let message = msg.unwrap();
                        match message {
                            Message::NewOrder(callbutton) => {
                                if self.state.behaviour == ElevatorBehaviour::Idle {
                                    self.next_order = callbutton;
                                    dprintln!("[SLAVE]\t floor: {:#?}, next_order: {:#?}", self.state.floor, self.next_order.floor);
                                    if self.state.floor == self.next_order.floor {
                                        self.set_behaviour(ElevatorBehaviour::DoorOpen);
                                        self.elevator.door_light(true);
                                        slave_inputs::start_timer(self.door_timer.0.clone(), self.config.door_open_duration_s);
                                        self.send_order_complete();
                                    }
                                    else {
                                        self.start_moving_normal();
                                    }
                                }
                                else {
                                   dprintln!("[SLAVE]\t\tReceived new order, but elevator is not idle");
                                }
                            },
                            Message::LightMatrix(matrix) => {
                                self.light_matrix = matrix;
                                self.sync_hall_lights();
                                dprintln!("[SLAVE]\t\tReceived light matrix");
                            },
                            // Receive state update from master. Used to syncronize the state of the elevator when connecting to a new master
                            Message::StateUpdate(state) => {
                                for i in 0..NUMBER_OF_FLOORS {
                                    if state.cab_requests[i] {
                                        self.state.cab_requests[i] = state.cab_requests[i];
                                    }
                                }
                                self.send_state_update();
                                dprintln!("[SLAVE]\t\tReceived state update");
                            },
                            Message::Error(_) => {
                                dprintln!("[SLAVE]\t\tReceived error message from master");
                                dprintln!("[SLAVE]\t\tStarting in local operating mode");
                                self.master_channels = None;

                                // Turn off all hall lights since we are in local mode and no longer take hall orders
                                for i in 0..NUMBER_OF_FLOORS {
                                    self.elevator.call_button_light(i as u8, HALL_UP, false);
                                    self.elevator.call_button_light(i as u8, HALL_DOWN, false);
                                }
                                if self.state.behaviour == ElevatorBehaviour::Idle{
                                    self.start_moving_local();
                                }

                            },
                            _ => {},   // Do nothing for OrderComplete messages and other messages
                        }
                    }
                    default(Duration::from_millis(self.config.input_poll_rate_ms*100)) => {
                        if self.state.behaviour == ElevatorBehaviour::Idle {
                            self.send_state_update();
                        }
                    }
                } // cbc::select
            } // if master_channels.is_some()


            /************** local operation mode ***************/
            else {
                cbc::select! {
                    // Receive floor sensor message from elevator
                    recv(self.channels.floor_sensor_rx) -> msg => {
                        let floor_sensor = msg.unwrap();
                        dprintln!("[SLAVE]\t\tReceived floor sensor message: {:#?}", floor_sensor);

                        slave_inputs::start_timer(self.motor_timeout.0.clone(), self.config.est_moving_time_s);
                        self.timestamp_prev_floor = std::time::Instant::now();

                        self.state.floor = floor_sensor;

                        if self.state.behaviour == ElevatorBehaviour::Moving {
                            self.state.floor = floor_sensor;
                            self.elevator.floor_indicator(self.state.floor);
                            if self.should_stop() {
                                dprintln!("[SLAVE]\t\tStopping at floor {:?}", self.state.floor);
                                self.set_behaviour(ElevatorBehaviour::DoorOpen);
                                self.elevator.door_light(true);
                                self.clear_at_current_floor();
                                self.sync_cab_lights();
                                self.elevator.motor_direction(DIRN_STOP);

                                slave_inputs::start_timer(self.door_timer.0.clone(), self.config.door_open_duration_s);
                            }
                        }
                    }

                    // Receive call button message from elevator
                    recv(self.channels.call_button_rx) -> msg => {
                        let call_button = msg.unwrap();
                        dprintln!("[SLAVE]\t\tReceived call button message: {:#?}", call_button);


                        // Update local cab requests
                        if call_button.call == CAB {
                            self.state.cab_requests[call_button.floor as usize] = true;
                        }

                        self.sync_cab_lights();

                        if self.state.behaviour == ElevatorBehaviour::Idle {
                                self.start_moving_local();
                        }
                    }

                    // Receive stop button message from elevator
                    recv(self.channels.stop_button_rx) -> msg => {
                        self.stop_button = msg.unwrap();
                        dprintln!("[SLAVE]\t\tStop button: {:#?}", self.stop_button);
                        if self.stop_button {
                            self.elevator.motor_direction(DIRN_STOP);
                            self.set_behaviour(ElevatorBehaviour::OutOfOrder);
                            return;
                        }
                        else {
                            self.set_behaviour(ElevatorBehaviour::Idle);
                        }
                    }

                    // Receive obstruction message from elevator
                    recv(self.channels.obstruction_rx) -> msg => {
                        let obstr = msg.unwrap();
                        self.obstruction = obstr;

                        dprintln!("[SLAVE]\t\tObstruction: {:#?}", obstr);
                    }

                    // Receive motor timeout if the elevator has not reached a floor within the estimated moving time
                    recv(self.motor_timeout.1) -> _msg => {
                        if      self.timestamp_prev_floor + Duration::from_secs(self.config.est_moving_time_s) < std::time::Instant::now()
                            &&  self.state.behaviour == ElevatorBehaviour::Moving
                            &&  !self.stop_button
                        {
                            dprintln!("[SLAVE]\t\tMotor timeout. Out of order.");
                            self.set_behaviour(ElevatorBehaviour::OutOfOrder);
                        }
                    }

                    // Receive timer message
                    recv(self.door_timer.1) -> _msg => {
                        if self.obstruction {
                            slave_inputs::start_timer(self.door_timer.0.clone(), self.config.door_open_duration_s);
                        }
                        else {
                            dprintln!("[SLAVE]\t\tTimer expired. Door closing.");
                            self.elevator.door_light(false);
                            self.set_behaviour(ElevatorBehaviour::Idle);
                            self.start_moving_local();
                        }
                    }
                    default(Duration::from_millis(self.config.input_poll_rate_ms)) =>  self.try_connect_to_new_master(),
                } // cbc::select
            } // else
        } // loop
    } // slave_loop

    /************ functions for local operation mode **************/

    fn orders_above(&mut self) -> bool {
        for floor in (self.state.floor + 1)..NUMBER_OF_FLOORS as u8 {
            if self.state.cab_requests[floor as usize] {
                self.next_order = CallButton { floor, call: CAB };
                return true;
            }
        }
        false
    }

    fn orders_below(&mut self) -> bool {
        for floor in 0..self.state.floor {
            if self.state.cab_requests[floor as usize] {
                self.next_order = CallButton { floor, call: CAB };
                return true;
            }
        }
        false
    }

    pub fn orders_here(&self) -> bool {
        self.state.cab_requests[self.state.floor as usize]
    }

    fn should_stop(&mut self) -> bool {
        match self.state.direction {
            Direction::Down => {
                self.state.cab_requests[self.state.floor as usize] || !self.orders_below()
            }
            Direction::Up => {
                self.state.cab_requests[self.state.floor as usize] || !self.orders_above()
            }
            _ => true,
        }
    }

    /// Choose direction based on orders and return the direction and behaviour
    fn choose_direction(&mut self) -> (Direction, ElevatorBehaviour) {
        match self.state.direction {
            Direction::Up => {
                if self.orders_above() {
                    (Direction::Up, ElevatorBehaviour::Moving)
                } else if self.orders_here() {
                    (Direction::Down, ElevatorBehaviour::DoorOpen)
                } else if self.orders_below() {
                    (Direction::Down, ElevatorBehaviour::Moving)
                } else {
                    (Direction::Stop, ElevatorBehaviour::Idle)
                }
            }

            Direction::Down => {
                if self.orders_below() {
                    (Direction::Down, ElevatorBehaviour::Moving)
                } else if self.orders_here() {
                    (Direction::Up, ElevatorBehaviour::DoorOpen)
                } else if self.orders_above() {
                    (Direction::Up, ElevatorBehaviour::Moving)
                } else {
                    (Direction::Stop, ElevatorBehaviour::Idle)
                }
            }

            Direction::Stop => {
                if self.orders_here() {
                    (Direction::Stop, ElevatorBehaviour::DoorOpen)
                } else if self.orders_above() {
                    (Direction::Up, ElevatorBehaviour::Moving)
                } else if self.orders_below() {
                    (Direction::Down, ElevatorBehaviour::Moving)
                } else {
                    (Direction::Stop, ElevatorBehaviour::Idle)
                }
            }
        }
    }

    /// Clear the cab request at the current floor
    fn clear_at_current_floor(&mut self) {
        self.state.cab_requests[self.state.floor as usize] = false;
    }

    fn start_moving_local(&mut self) {
        let (diraction, behaviour) = self.choose_direction();
        self.next_order = CallButton {
            floor: 1,
            call: CAB,
        };
        self.state.behaviour = behaviour;

        slave_inputs::start_timer(self.motor_timeout.0.clone(), self.config.est_moving_time_s);
        self.timestamp_prev_floor = Instant::now();

        if behaviour == ElevatorBehaviour::DoorOpen {
            dprintln!("Stopped with door open at floor {:?}", self.state.floor);
            self.clear_at_current_floor();
            self.sync_cab_lights();
            self.elevator.door_light(true);
            slave_inputs::start_timer(self.door_timer.0.clone(), self.config.door_open_duration_s);
        }

        match diraction {
            Direction::Up => {
                self.elevator.motor_direction(DIRN_UP);
                self.state.direction = Direction::Up;
            }
            Direction::Down => {
                self.elevator.motor_direction(DIRN_DOWN);
                self.state.direction = Direction::Down;
            }
            Direction::Stop => {
                self.elevator.motor_direction(DIRN_STOP);
                self.state.direction = Direction::Stop;
            }
        }
    }
}

impl Display for Slave {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Slave:\n\
            \tElevator:\t{:#?}\n\
            \tState:\t{:#?}\n\
            \tnext_order:\t{:#?}\n\
            \tObstruction:\t{:#?}\n\
            \tChannels:\t{:#?}\n\
            \tMaster_socket:\t{:#?}\n\
            \tDoor_timer:\t{:#?}",
            self.elevator,
            self.state,
            self.next_order,
            self.obstruction,
            self.channels,
            self.master_channels,
            self.door_timer
        )
    }
}
