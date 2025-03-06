use crossbeam_channel as cbc;
use driver_rust::elevio::{self};
use std::fmt;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;

use crate::tcp;

// Struct containing all the rx channels from the elevator io driver. 
#[derive(Debug, Clone)]
pub struct SlaveChannels {
    pub floor_sensor_rx: cbc::Receiver<u8>,
    pub call_button_rx: cbc::Receiver<elevio::poll::CallButton>,
    pub stop_button_rx: cbc::Receiver<bool>,
    pub obstruction_rx: cbc::Receiver<bool>,
    pub master_message_rx: cbc::Receiver<tcp::Message>,
}

// Spawns threads for all the slave input channels and returns a SlaveChannels struct. 
pub fn spawn_threads_for_slave_inputs(
    elevator: &elevio::elev::Elevator,
    input_poll_rate_ms: u64,
    master_socket: &TcpStream,
) -> SlaveChannels {
    let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);

    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::call_buttons(elevator, call_button_tx, poll_period));
    }

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period));
    }

    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::stop_button(elevator, stop_button_tx, poll_period));
    }

    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::obstruction(elevator, obstruction_tx, poll_period));
    }

    let mut master_socket_clone = master_socket.try_clone().expect("Failed to clone socket");
    let (master_message_tx, master_message_rx) = cbc::unbounded::<tcp::Message>();
    spawn(move || {
        let mut encoded = [0; 1024];
        loop {
            match master_socket_clone.read(&mut encoded) {
                Ok(size) => {
                    if size > 0 {
                        let message: tcp::Message =
                            bincode::deserialize(&encoded).expect("Failed to deserialize message");
                        println!("[SLAVE]\tReceived message from master: {:#?}", message);
                        master_message_tx.send(message).unwrap();
                    }
                }
                Err(e) => {
                    println!("[SLAVE]\tFailed to read from stream: {}", e);
                    continue; // TODO: Sjekk om dette er riktig
                              // return e;
                }
            }
            sleep(poll_period);
        }
    });

    SlaveChannels {
        floor_sensor_rx,
        call_button_rx,
        stop_button_rx,
        obstruction_rx,
        master_message_rx,
    }
}


impl fmt::Display for SlaveChannels {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SlaveChannels {{
    floor_sensor_rx: {:?},
    call_button_rx: {:?},
    stop_button_rx: {:?},
    obstruction_rx: {:?},
    master_message_rx: {:?}
}}",
            self.floor_sensor_rx,
            self.call_button_rx,
            self.stop_button_rx,
            self.obstruction_rx,
            self.master_message_rx
        )
    }
}

/********************************************************************************************************************/


#[derive(Debug, Clone)]
pub struct MasterChannels {
    pub slave_vector_rx: Vec<cbc::Receiver<tcp::Message>>,
    pub backup_rx: cbc::Receiver<tcp::Message>,
}

//Listen for new slave connections. Is runned in separate thread.
// Need error handling
pub fn listen_for_new_connection(port: &String) -> Option<TcpStream> {
    let listener = TcpListener::bind("0.0.0.0".to_string() + ":" + port).expect("Failed to bind");
    println!("Listening for new connection\n");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                //stream.set_read_timeout(Some(Duration::from_secs(tcp_timeout))).expect("Failed to set read timeout");
                return Some(stream);
            }
            Err(e) => {
                println!("[MASTER]\tFailed to establish connection: {}", e);
                return None;
            }
        }
    }
    None
}


//Not used anywher. Under development or not needed?
pub fn master_read_from_clients(
    mut stream: TcpStream,
    input_poll_rate_ms: u64,
) -> cbc::Receiver<tcp::Message> {
    let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);

    let (tx, rx) = cbc::unbounded::<tcp::Message>();
    spawn(move || {
        let mut encoded = [0; 1024];
        loop {
            match stream.read(&mut encoded) {
                Ok(size) => {
                    if size > 0 {
                        let message: tcp::Message =
                            bincode::deserialize(&encoded).expect("Failed to deserialize message");
                        println!("[MASTER]\tReceived message from client: {:#?}", message);
                        tx.send(message).unwrap();
                    }
                }
                Err(e) => {
                    println!("[MASTER]\tFailed to read from tcp-stream: {}", e);
                    continue; // TODO: Check if this is correct. Maybe need to return something to scheck if the connection is lost. 
                }
            }
            sleep(poll_period);
        }
    });
    rx
}


