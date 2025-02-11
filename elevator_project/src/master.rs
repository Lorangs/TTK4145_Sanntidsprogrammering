use std::{thread, time};
use std::process::Command;
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::{tcp, slave};

const ADDRESS: &str = "127.0.0.1:4000";


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Queues {
    pub main_queue: VecDeque<Vec<u8>>,
    pub slave_queues: Vec<VecDeque<u8>>,
}

impl Queues {
    pub fn new() -> Self {
        Self {
            main_queue: VecDeque::new(),
            slave_queues: vec![VecDeque::new(), VecDeque::new(), VecDeque::new()],
        }
    }

    pub fn add_to_main_que(&mut self, floor: u8, direction: u8) {
        self.main_queue.push_back(vec![floor, direction]);
    }

    pub fn add_to_slave_que(&mut self, slave: usize, floor: u8) {
        if slave < self.slave_queues.len() {
            self.slave_queues[slave].push_back(floor);
        } else {
            println!("Error: Slave queue index {} is out of bounds!", slave);
        }
    }

    pub fn print_que(&self) {
        println!("Main Queue:");
        for (index, item) in self.main_queue.iter().enumerate() {
            println!("  Place {}: {:?}", index, item);
        }
        println!("\nSlave Queues:");
        for (index, item) in self.slave_queues.iter().enumerate() {
            println!("  Slave {}: {:?}", index, item);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}

#[derive(Debug, Clone)]
struct Master {
    pub master_ip   :         String,                                                   // IP address of master
    pub backup_ip   :         String,                                                   // IP address of backup
    pub slaves_ip   :         [String; NUMBER_OF_ELEVATORS ],                           // Vector of slaves IP addresses
    pub slaves_order:         [[Order; NUMBER_OF_FLOORS]; NUMBER_OF_ELEVATORS]          // Vector of slaves order queues  
}


impl Master {
    pub fn init(config: Config) -> Master {
        Self  {
            master_ip       : config.elevator_ip_list[0],                               // IP address of master
            backup_ip       : config.elevator_ip_list[1],                               // IP address of backup
            slaves_ip       : config.elevator_ip_list[0..],                             // Vector of slaves IP addresses                 
            slaves_order    : [Order
                                    {
                                        hall_down   : false,
                                        hall_up     : false,
                                        cab_call    : false,    
                                    }; NUMBER_OF_ELEVATORS],
        }
    }
    pub fn backup_task(queues: Arc<Mutex<Queues>>) {
        println!("Trying to connect to a primary at {}", ADDRESS);
        match TcpStream::connect(ADDRESS) {
            Ok(stream) => {
                println!("Connected to primary, acting as backup.");
                let reader = BufReader::new(stream);

                for line in reader.lines() {
                    match line {
                        Ok(msg) => {
                            println!("Backup: Received full message: {:?}", msg);
                            match serde_json::from_str::<Message>(&msg) {
                                Ok(Message::Queues(q)) => {
                                    *queues.lock().unwrap() = q;
                                    queues.lock().unwrap().print_que();
                                }
                                Ok(_) => println!("Backup: Received a non-queue message."),
                                Err(e) => println!("Backup: Failed to parse JSON: {}", e),
                            }
                        }
                        Err(_) => {
                            println!("Backup: Connection lost, becoming primary");
                            return;
                        }
                    }
                    thread::sleep(time::Duration::from_secs(1));
                }
            }
            Err(_) => {
                println!("Could not connect to a primary... becoming primary");
                return;
            }
        }
    }

    pub fn primary_task(mut stream: TcpStream, queues: Arc<Mutex<Queues>>) {
        let mut writer = BufWriter::new(&stream);

        loop {
            let mut q = queues.lock().unwrap();
            q.add_to_main_que(1, 1);
            q.add_to_slave_que(1, 3);

            let message = serde_json::to_string(&Message::Queues(q.clone())).unwrap();

            match writeln!(writer, "{}", message) {
                Ok(_) => writer.flush().unwrap(),
                Err(_) => {
                    println!("Primary: Backup disconnected.");
                    break;
                }
            }

            println!("Primary: Updated backup with current queue:");
            q.print_que();
            thread::sleep(time::Duration::from_secs(3));
        }
    }
}

fn main() {
    let queues: Arc<Mutex<Queues>> = Arc::new(Mutex::new(Queues::new()));

    // Try to be backup
    backup(queues.clone());

    // If backup fails, become primary
    let listener = TcpListener::bind(ADDRESS).expect("Could not start TCP server");
    println!("Primary: Listening on {}", ADDRESS);

    loop {
        println!("Primary: Starting a backup...");

        Command::new("cargo")
            .args(["run", "--bin", "test"])
            .spawn()
            .expect("Failed to start backup");

        if let Ok((stream, _)) = listener.accept() {
            println!("Primary: Backup connected!");
            primary_task(stream, queues.clone());
        }

        println!("Primary: Backup has exited, starting a new one.");
    }
}
 */