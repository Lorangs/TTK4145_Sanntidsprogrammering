use std::{thread, time};
use std::io::BufRead;
use std::io::BufReader;
use std::net::TcpStream;
use std::process::Command;
use std::net::TcpListener;
use std::io::Write;



const ADDRESS: &str = "127.0.0.1:4000";

fn main() {
    let mut number=0;
    println!("A: Trying to connect to B at {}", ADDRESS);
    match TcpStream::connect(ADDRESS) {
        Ok(stream) => {
            println!("A: Connected to B!");

            let reader = BufReader::new(stream);
            for line in reader.lines() {
                match line {
                    Ok(msg) => {println!("A received: {}", msg);
                                number = msg.parse().expect("Not a valid number");
                                println!("B received:{}", number);   },
                    Err(_) => {
                        println!("A: Connection lost, exiting...");
                        break;
                    }
                }
                thread::sleep(time::Duration::from_secs(3)); // Simulate processing time
            }
        }
        Err(_) => {
            println!("A: Could not connect to B, exiting...");
        }
    }
    println!("A: Starting a new B...");
    let mut child = Command::new("cargo")
            .args(["run", "--bin", "b"])
            .spawn()
            .expect("Failed to restart B");

    let listener = TcpListener::bind(ADDRESS).expect("Could not start TCP server");
    if let Ok((mut stream, _)) = listener.accept() {
        match writeln!(stream, "{}", number) { //writeln returnera om den klarte det eller ikkje
            Ok(_) => {println!("A: Sent where we stopped{}", number);} 
            Err(_) => {
                println!("A: B disconnected.");
            }
        }
    }
    let _ = child.wait();
    println!("A: Exseting.");
}
