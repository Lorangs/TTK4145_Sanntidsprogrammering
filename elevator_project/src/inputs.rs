use crate::config::BUFFER_SIZE;
use crate::io_datastructures::{ErrorState, Message};
use crossbeam_channel::{self as cbc};
use debug_print::debug_println as dprintln;
use driver_rust::elevio::{self};
use std::fmt::{Display as FmtDisplay, Formatter as FmtFormatter, Result as FmtResult};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::{sleep, spawn};
use std::time::Duration;

// Struct containing all the rx channels from the elevator io driver to the Slave unit. 
#[derive(Debug, Clone)]
pub struct SlaveChannels {
    pub floor_sensor_rx: cbc::Receiver<u8>,
    pub call_button_rx: cbc::Receiver<elevio::poll::CallButton>,
    pub stop_button_rx: cbc::Receiver<bool>,
    pub obstruction_rx: cbc::Receiver<bool>,
}
impl FmtDisplay for SlaveChannels {
    fn fmt(&self, f: &mut FmtFormatter) -> FmtResult {
        write!(
            f,
            "SlaveChannels:\n\t
            floor_sensor_rx:\t{:?}\n\t
            call_button_rx:\t{:?}\n\t
            stop_button_rx:\t{:?}\n\t
            obstruction_rx:\t{:?}",
            self.floor_sensor_rx,
            self.call_button_rx,
            self.stop_button_rx,
            self.obstruction_rx,

        )
    }
}

/// Spawns threads for all the slave input channels and returns a SlaveChannels struct. 
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

/// Spawns a thread for channels handling the TcpStream connection to the master.
/// Returns a tuple with the sender and receiver for the channels.
pub fn spawn_thread_for_master_connection
(
    mut stream: TcpStream,
    input_poll_rate_ms: u64,
) -> (cbc::Sender<Message>, cbc::Receiver<Message>) {
    let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);
    let (master_to_slave_tx, master_to_slave_rx) = cbc::unbounded::<Message>();
    let (slave_to_master_tx, slave_to_master_rx) = cbc::unbounded::<Message>();

    stream.set_nonblocking(true).expect("Failed to set non-blocking mode on stream");
    stream.set_nodelay(true).expect("Failed to set nodelay"); // Gjør store forbedringer i ytelse. Må være true
    stream.set_ttl(3).expect("Failed to set ttl");

    spawn(move || {
        let mut encoded: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
        loop {
            match slave_to_master_rx.try_recv() {
                Ok(message) => {
                    let encoded =
                        bincode::serialize(&message).expect("Failed to serialize message");
                    match stream.write_all(&encoded) {
                        Ok(_) => {
                            dprintln!("[SLAVE]\t\tSent message to master: {:#?}", message);
                        }
                        Err(e) => {
                            dprintln!("[SLAVE]\t\tFailed to write to stream: {}", e);
                            master_to_slave_tx
                                .send(Message::Error(ErrorState::Network))
                                .unwrap();
                        }
                    }
                }
                Err(_e) => {
                    // No message received on channel. Continue.
                    continue;
                }
            }

            match stream.read(&mut encoded) {
                Ok(size) => {
                    if size > 0 {
                        let msg: Message = bincode::deserialize::<Message>(&encoded[..size])
                            .expect("Failed to deserialize message");
                        dprintln!("[SLAVE]\t\tReceived message from master: {:#?}", msg);
                        master_to_slave_tx.send(msg).unwrap();
                    }
                }
                Err(e) => {
                    match e.kind() {
                        // WouldBlock is expected when no data is available to read.
                        std::io::ErrorKind::WouldBlock => {},

                        // Treat all other errors as network errors.
                        _ => {
                            dprintln!("[SLAVE]\t\tFailed to read from stream: {}", e);
                            master_to_slave_tx
                                .send(Message::Error(ErrorState::Network))
                                .unwrap();
                        }
                    }
                }
            }
            sleep(poll_period); //eg vil teste uten sleep hær
        }
    });
    (slave_to_master_tx, master_to_slave_rx)
}


/// Spawn a new thread that will sleep for the given duration and then send a message to the door_timer channel when done. 
pub fn start_timer(tx: cbc::Sender<bool>, duration: u64) {
    spawn(move || {
        sleep(Duration::from_secs(duration));
        let _ = tx.send(true).unwrap();
    });
}