use crossbeam_channel::{self as cbc};
use driver_rust::elevio::{self};
use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::Duration;
use crate::tcp;

// Struct containing all the rx channels from the elevator io driver. 
#[derive(Debug, Clone)]
pub struct SlaveChannels {
    pub floor_sensor_rx : cbc::Receiver<u8>,
    pub call_button_rx  : cbc::Receiver<elevio::poll::CallButton>,
    pub stop_button_rx  : cbc::Receiver<bool>,
    pub obstruction_rx  : cbc::Receiver<bool>,
}

// Spawns threads for all the slave input channels and returns a SlaveChannels struct. 
pub fn spawn_threads_for_slave_inputs
(
    elevator: &elevio::elev::Elevator,
    input_poll_rate_ms: u64,
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

    SlaveChannels {
        floor_sensor_rx,
        call_button_rx,
        stop_button_rx,
        obstruction_rx,
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

}}",
            self.floor_sensor_rx,
            self.call_button_rx,
            self.stop_button_rx,
            self.obstruction_rx,

        )
    }
}


pub fn spawn_thread_for_master_connection
(
    mut stream: TcpStream,
    input_poll_rate_ms: u64,
) -> (cbc::Sender<tcp::Message>, cbc::Receiver<tcp::Message>)
{
    let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);
    let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded::<tcp::Message>();
    let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded::<tcp::Message>();

    //stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
    stream.set_read_timeout(Some(poll_period)).expect("Failed to set read timeout");
    stream.set_write_timeout(Some(poll_period)).expect("Failed to set write timeout");

    spawn(move || {
        let mut encoded = [0; 1024];
        loop {
            match slave_to_master_rx.try_recv() {
                Ok(message) => {
                    let encoded = bincode::serialize(&message).expect("Failed to serialize message");
                    match stream.write(&encoded) {
                        Ok(_) => {
                            println!("[SLAVE]\t\tSent message to master: {:#?}", message);
                        }
                        Err(e) => {
                            println!("[SLAVE]\t\tFailed to write to stream: {}", e);
                            master_to_slave_tx.send(tcp::Message::Error(tcp::ErrorState::Network)).unwrap();
                        }
                    }
                }
                Err(_e) => {
                    //println!("[SLAVE]\t\tFailed to receive message from channel: {}", e);
                    continue;
                }
            }

            match stream.read(&mut encoded) {
                Ok(size) => {
                    if size > 0 {
                        let message: tcp::Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                        println!("[SLAVE]\t\tReceived message from master: {:#?}", message);
                        master_to_slave_tx.send(message).unwrap();
                    }
                }
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            // println!("[SLAVE]\t\tNo data available");
                        }
                        _ => {
                            println!("[SLAVE]\t\tFailed to read from stream: {}", e);
                            master_to_slave_tx.send(tcp::Message::Error(tcp::ErrorState::Network)).unwrap();
                        }
                    }
                }
            }
            sleep(poll_period);
        }
    });
    (slave_to_master_tx, master_to_slave_rx)
}


/********************************************************************************************************************/
/*********Master Inputs**********/

#[derive(Debug, Clone)]
pub struct MasterChannels {
    pub slave_vector_rx: Vec<cbc::Receiver<tcp::Message>>,
    pub backup_rx: cbc::Receiver<tcp::Message>,
}


pub fn listen_for_new_connection(port: &String) -> Option<TcpStream> {
    let listener = TcpListener::bind("0.0.0.0".to_string() + ":" + port).expect("Failed to bind");
    println!("[MASTER]\tListening for new connection");
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


