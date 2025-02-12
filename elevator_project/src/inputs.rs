use crossbeam_channel as cbc;
use std::fmt;
use std::thread::{spawn, sleep};
use std::time::Duration;
use std::net::TcpStream;
use std::io::Read;  
use driver_rust::elevio::{self};

use crate::{slave, config, tcp};

#[derive(Debug, Clone)]
pub struct SlaveChannels {
    pub floor_sensor_rx     : cbc::Receiver<u8>,
    pub call_button_rx      : cbc::Receiver<elevio::poll::CallButton>,
    pub stop_button_rx      : cbc::Receiver<bool>, 
    pub obstruction_rx      : cbc::Receiver<bool>,
    pub master_message_rx   : cbc::Receiver<tcp::Message>,
}

pub fn spawn_threads_for_slave_inputs(elevator: &elevio::elev::Elevator, input_poll_rate_ms: u64, master_socket: &TcpStream) -> SlaveChannels {
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
        loop{
            match master_socket_clone.read(&mut encoded) {
                Ok(size) => {
                    if size > 0 {
                        let message: tcp::Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                        println!("[SLAVE]\tReceived message from master: {:#?}", message);
                        master_message_tx.send(message).unwrap();
                    }
                }
                Err(e) => {
                    println!("[SLAVE]\tFailed to read from stream: {}", e);
                    continue;               // TODO: Sjekk om dette er riktig
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



#[derive(Debug, Clone)]
pub struct MasterChannels {
    pub slave_vector_rx    : Vec<cbc::Receiver<tcp::Message>>,
    pub backup_rx          : cbc::Receiver<tcp::Message>,

}


// TODO: Implement this function and rename 
pub fn handle_master_connections(stream: &mut TcpStream, input_poll_rate_ms: u64) -> cbc::Receiver<tcp::Message> {
    let (master_tx, master_rx) = cbc::unbounded::<tcp::Message>();
    spawn(move || {
        let mut encoded = [0; 1024];
        loop{
            match stream.read(&mut encoded) {
                Ok(size) => {
                    if size > 0 {
                        let message: tcp::Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                        println!("[MASTER]\tReceived message from client: {:#?}", message);
                        master_tx.send(message).unwrap();
                    }
                }
                Err(e) => {
                    println!("[MASTER]\tFailed to read from tcp-stream: {}", e);
                    continue;               // TODO: Sjekk om dette er riktig
                    // return e;
                }
            }            
            let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);
            sleep(poll_period);
        }
    });
    master_rx
}


pub fn spawn_threads_for_master_inputs(, input_poll_rate_ms: u64) -> MasterChannels {
    let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);  

    // slave_vec_rx is a vector of receivers, one for each slave
}

/* 

pub fn spawn_threads_for_master_inputs(slave_sockets: &Vec<TcpStream>, backup_socket: &TcpStream, input_poll_rate_ms: u64) -> MasterChannels {
    let poll_period: Duration = Duration::from_millis(input_poll_rate_ms);  

    // slave_vec_rx is a vector of receivers, one for each slave
    let mut slave_vec_rx: Vec<cbc::Receiver<tcp::Message>> = Vec::new();

    // loop over all slaves and spawn a new thread for each slave socket
    for (index, socket) in slave_sockets.iter().enumerate() {

        let (slave_message_tx, slave_message_rx) = cbc::unbounded::<tcp::Message>();
        let mut socket_clone = socket.try_clone().expect("Failed to clone socket");
        spawn(move || {
            let mut encoded = [0; 1024];        // bit array to store the encoded message
            loop{
                // read from the socket into the encoded array
                match socket_clone.read(&mut encoded) {
                    Ok(size) => {
                        if size > 0 {
                            let message: tcp::Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                            println!("[MASTER]\tReceived message from slave {}: {:#?}", index, message);
                            slave_message_tx.send(message).unwrap();
                        }
                    }
                    Err(e) => {
                        println!("[MASTER]\tFailed to read from stream: {}", e);
                        continue;               // TODO: Sjekk om dette er riktig
                        // return e;
                    }
                }            
                sleep(poll_period);
            }
        });
        slave_vec_rx.push(slave_message_rx);
    }


    // spawn a new thread for the backup socket
    let mut backup_socket_clone = backup_socket.try_clone().expect("Failed to clone socket");
    let (backup_tx, backup_rx) = cbc::unbounded::<tcp::Message>();
    spawn(move || {
        let mut encoded = [0; 1024];
        loop{
            match backup_socket_clone.read(&mut encoded) {
                Ok(size) => {
                    if size > 0 {
                        let message: tcp::Message = bincode::deserialize(&encoded).expect("Failed to deserialize message");
                        println!("[MASTER]\tReceived message from backup: {:#?}", message);
                        backup_tx.send(message).unwrap();
                    }
                }
                Err(e) => {
                    println!("[MASTER]\tFailed to read from stream: {}", e);
                    continue;               // TODO: Sjekk om dette er riktig
                    // return e;
                }
            }            
            sleep(poll_period);
        }
    });

    MasterChannels {
        slave_vector_rx : slave_vec_rx,
        backup_rx       : backup_rx,
    }
} 

*/