use crossbeam_channel::{self as cbc};
use driver_rust::elevio::{self};
use std::fmt::{Display as FmtDisplay, Result as FmtResult, Formatter as FmtFormatter};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::{sleep, spawn};
use std::time::Duration;
use crate::tcp::{Message, ErrorState};

// Struct containing all the rx channels from the elevator io driver. 
#[derive(Debug, Clone)]
pub struct SlaveChannels {
    pub floor_sensor_rx : cbc::Receiver<u8>,
    pub call_button_rx  : cbc::Receiver<elevio::poll::CallButton>,
    pub stop_button_rx  : cbc::Receiver<bool>,
    pub obstruction_rx  : cbc::Receiver<bool>,
}

impl FmtDisplay for SlaveChannels {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
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


pub fn spawn_thread_for_master_connection
(
    mut stream: TcpStream,
    input_poll_rate_ms: u64,
) -> (cbc::Sender<Message>, cbc::Receiver<Message>)
{
    let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);
    let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded::<Message>();
    let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded::<Message>();

    //stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
    stream.set_read_timeout(Some(poll_period)).expect("Failed to set read timeout");
    stream.set_write_timeout(Some(poll_period)).expect("Failed to set write timeout");

    spawn(move || {
        let mut encoded: [u8; 1024] = [0; 1024];
        loop {
            match slave_to_master_rx.try_recv() {
                Ok(message) => {
                    let encoded: Vec<u8> = bincode::serialize(&message).expect("Failed to serialize message");
                    match stream.write(&encoded) {
                        Ok(_) => {
                            println!("[SLAVE]\t\tSent message to master: {:#?}", message);
                        }
                        Err(e) => {
                            println!("[SLAVE]\t\tFailed to write to stream: {}", e);
                            master_to_slave_tx.send(Message::Error(ErrorState::Network)).unwrap();
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
                        let msg: Message = bincode::deserialize::<Message>(&encoded).expect("Failed to deserialize message");
                        //println!("[SLAVE]\t\tReceived message from master: {:#?}", msg);
                        master_to_slave_tx.send(msg).unwrap();
                    }
                }
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::WouldBlock => {
                            // println!("[SLAVE]\t\tNo data available");
                        }
                        _ => {
                            println!("[SLAVE]\t\tFailed to read from stream: {}", e);
                            master_to_slave_tx.send(Message::Error(ErrorState::Network)).unwrap();
                        }
                    }
                }
            }
            sleep(poll_period);
        }
    });
    (slave_to_master_tx, master_to_slave_rx)
}
