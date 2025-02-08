use std::{thread, time};
use std::process::Command;
use std::io::{Write, BufReader, BufRead};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

const ADDRESS: &str = "127.0.0.1:4000";

fn handle_client(mut stream: TcpStream, number: Arc<Mutex<i32>>)->bool {
    for _i in 1..20 {
        let mut num = number.lock().unwrap();
        *num +=1;
        match writeln!(stream, "{}", *num) { //writeln returnera om den klarte det eller ikkje
            Ok(_) => {} // Do nothing if writing succeeds
            Err(_) => {
                println!("Primary: Backup disconnected.");
                return false; // Exit the loop if writing fails
            }
        }
        println!("Primary: Sent {}", num);
        if *num >=7{
            println!("Primary: Reached max value, exiting...");
            return true;
        }
        thread::sleep(time::Duration::from_secs(1));
    }
    return false;
}

fn main(){
    let number = Arc::new(Mutex::new(0));
    //chek if i was started by another program, if yes act as backup

    println!("Trying to connect to a primary at {}", ADDRESS);
    match TcpStream::connect(ADDRESS) {
        Ok(stream) => {
            println!("Conected to primary, acting as backup.");

            let reader = BufReader::new(stream);
            for line in reader.lines() {
                match line {
                    Ok(msg) => {println!("Backup: Received: {}", msg);
                                let mut num = number.lock().unwrap();
                                *num = msg.parse().expect("Not a valid number"); 
                                if *num >=5 {
                                    println!("Backup: reched my goal so I stop");
                                    return;
                                }
                                },
                    Err(_) => {
                        println!("Backup: Connection lost, exiting and becoming primary");
                        break;
                    }
                }
                thread::sleep(time::Duration::from_secs(1)); // Simulate processing time
            }
            println!("Backup: No more lines to reed, asuming primery is dear, take over");
        }
        Err(_) => {
            println!("Could not connect to a primary... becoming primary");
        }
    }

    //if not a backup or if i lost conecrion to my primary I become primary
    let listener = TcpListener::bind(ADDRESS).expect("Could not start TCP server");
    loop{

        println!("Primary: Listening on {}", ADDRESS);
        println!("Primary: Starting a backup...");

        let mut child = Command::new("cargo")
            .args(["run", "--bin", "combo"])
            .spawn()
            .expect("Failed to start backup");

        if let Ok((stream, _)) = listener.accept() {
            println!("Primary: Backup connected!");
            let dead=handle_client(stream, number.clone());
            if dead{
                println!("Primary: dead");
                return;
            }
        }

        let _ = child.wait();
        println!("Primary: Backup has exited starting a new one.");
        
    }
}