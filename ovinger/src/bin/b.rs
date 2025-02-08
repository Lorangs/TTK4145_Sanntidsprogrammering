use std::process::Command;
use std::{thread, time};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::io::BufReader;
use std::io::BufRead;




fn handle_client(mut stream: TcpStream, mut number: i32) {
    for i in 1..4 {
        let sending=number+i;
        match writeln!(stream, "{}", sending) { //writeln returnera om den klarte det eller ikkje
            Ok(_) => {} // Do nothing if writing succeeds
            Err(_) => {
                println!("B: A disconnected.");
                break; // Exit the loop if writing fails
            }
        }
        println!("B: Sent {}", i);
        thread::sleep(time::Duration::from_secs(3));
    }
}

fn main() {
    let address = "127.0.0.1:4000";
    let mut number: i32 = 0;

    match TcpStream::connect(address) {
        Ok(stream) => {
            println!("B: I was a restarted program!");

            let reader = BufReader::new(stream);
            for line in reader.lines() {
                match line {
                    Ok(msg) => {println!("B received: {}", msg);
                                number = msg.parse().expect("Not a valid number");
                                println!("B received:{}", number);  
                                },
                    Err(_) => {
                        println!("B: Connection to the last A lost, exiting and starting a new...");
                        break;
                    }
                }
                thread::sleep(time::Duration::from_secs(1)); // Simulate processing time
            }
        }
        Err(_) => {
            println!("B: Could not connect to a former A, exiting and starting a new....");
        }
    }


    let listener = TcpListener::bind(address).expect("Could not start TCP server");

    println!("B: Listening on {}", address);
    println!("B: Starting A...");

    let mut child = Command::new("cargo")
        .args(["run", "--bin", "a"])
        .spawn()
        .expect("Failed to start A");

    if let Ok((stream, _)) = listener.accept() {
        println!("B: A connected!");
        handle_client(stream, number);
    }

    let _ = child.wait();
    println!("B: B has exited.");
}


