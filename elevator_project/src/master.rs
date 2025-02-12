use std::thread::{spawn, sleep};
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::fmt::{Display as FmtDisplay, Formatter, Result as FmtResult};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};
use crate::{tcp, config, inputs};


#[derive(Serialize, Deserialize, Debug)]
pub struct MasterQueues {
    pub hall_queue: VecDeque<(u8, u8)>,     // (floor, button_type) for external hall calls.
    pub cab_queues: Vec<VecDeque<u8>>,      // Vector of slave queues for internal cab calls.  ref driver_rust::elevio::poll::CallButton
}

impl MasterQueues {
    pub fn init() -> MasterQueues {
        let hall_queue      : VecDeque<(u8, u8)>    = VecDeque::new();
        let cab_queues      : Vec<VecDeque<u8>>     = Vec::new();
        
        MasterQueues {
            hall_queue,
            cab_queues,
        }
    }

    pub fn add_to_hall_queue(&mut self, floor: u8, direction: u8) {
        self.hall_queue.push_back((floor, direction));
    }

    pub fn add_to_cab_queue(&mut self, slave_num: u8, floor: u8) {
        if self.cab_queues.len() > slave_num as usize {
            self.cab_queues[slave_num as usize].push_back(floor);
        } else {
            println!("Error: Slave queue index {} is out of bounds!", slave_num);
        }
    }
}

impl FmtDisplay for MasterQueues {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f, 
            "Hall queue: {:?}\n\
            Cab queues: {:?}", 
            self.hall_queue, 
            self.cab_queues)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}

#[derive(Debug)]
struct Master {
    pub config              : config::Config,                                           // Config struct
    pub master_ip           : String,                                                   // IP address of master
    pub backup_ip           : String,                                                   // IP address of backup
    pub slaves_ip           : Vec<String>,                                              // Vector of slaves IP addresses
    pub slaves_order        : MasterQueues,                                             // Vector of slaves order queues  

    //slave_sockets           : Vec<Option<TcpStream>>,                                   // Vector of slave sockets
    //backup_socket           : TcpStream,                                                // Backup socket
}


impl Master {
    pub fn init(
        config              : config::Config,
        master_ip           : String
    ) -> Result<Master, String> {

        let conf            : config::Config    = config.clone();
        let backup_ip       : String            = match config.elevator_ip_list.iter().find(|&ip| *ip != master_ip) {
                                                            Some(ip) => ip.to_string() + ":" + &config.backup_port.to_string(),
                                                            None     => return Err("No valid backup IP found".to_string())
                                                        };



        
        
        
        spawn(move || {
            loop {
                for stream in slave_listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            println!("Ny slave-tilkobling: {}", stream.peer_addr().unwrap());
                            inputs::handle_master_clients(stream, config.input_poll_rate_ms);
                            
                        }
                        Err(_) => { println!("Failed to connect to slave"); }
                    }
                }
                sleep(std::time::Duration::from_millis(config.input_poll_rate_ms));
            }
        });

        spawn(move || {
            loop {
                for mut stream in backup_listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            println!("Ny slave-tilkobling: {}", stream.peer_addr().unwrap());
                            inputs::handle_master_connections(&stream, config.input_poll_rate_ms);
                            
                        }
                        Err(_) => { println!("Failed to connect to slave"); }
                    }
                }
                sleep(std::time::Duration::from_millis(config.input_poll_rate_ms));
            }
        });
/*  
        let backup_socket   : TcpStream         = match TcpStream::connect(&backup_ip) {
                                                        Ok(socket)  => socket,
                                                        Err(_)          => return Err("Failed to connect to backup".to_string())
                                                    };

        let slave_sockets: Vec<Option<TcpStream>> = config.elevator_ip_list.iter().map(|ip| {
                                                                        match TcpStream::connect(ip.to_string() + ":" + &config.slave_port.to_string()) {
                                                                            Ok(socket)  => Some(socket),
                                                                            Err(_)          => None
                                                                        }
                                                                    }).collect();
 */
        Ok(Master {
            config          : conf,
            master_ip       : master_ip,                              // IP address of master
            backup_ip       : backup_ip,                              // IP address of backup
            slaves_ip       : config.elevator_ip_list,                // Vector of slaves IP addresses                 
            slaves_order    : MasterQueues::init(),                   // Vector of slaves order queues

            //slave_sockets   : slave_sockets,
        })
    }

    fn send_order_to_slave(&self, slave_num: u8, order: Order) {
        let message = tcp::Message::Order {
            slave_num,
            order,
        };
        let encoded = bincode::serialize(&message).unwrap();
        let mut sock = self.slave_sockets[slave_num as usize].as_ref().unwrap().try_clone().unwrap();
        sock.write(&encoded).unwrap();
    }

    fn master_loop(&mut self) {
        loop {
            cbc::select! {

                recv(self.slave_sockets[0].as_ref().unwrap(), message) -> msg => {
                    match msg {
                        Ok(message) => {
                            match message {
                                tcp::Message::Order {slave_num, order} => {
                                    self.slaves_order.add_to_cab_queue(slave_num, 1);
                                }
                                _ => {}
                            }
                        }
                        Err(_) => {}
                    }en potensielle problemer som kan påvirke dens funksjonalitet:
                }
            }
        }
    } 

    fn connect_to_clients(&self) {
        // Bør test om listner.incomig må kjøres i loop for å motta nye tilkoblinger. 
                                                        
        // Create listener socket for incoming slave connections. Listen on all interfaces.
        let slave_listener  : TcpListener       = match TcpListener::bind("0.0.0.0".to_string() + ":" + &config.slave_port.to_string()) {
            Ok(listener) => listener,
            Err(_)      => return Err("Failed to bind listener".to_string())
        };

        let backup_listener : TcpListener       = match TcpListener::bind("0.0.0.0".to_string() + ":" + &config.backup_port.to_string()) {
                                                            Ok(listener) => listener,
                                                            Err(_)      => return Err("Failed to bind listener".to_string())
                                                        };

    }


}

