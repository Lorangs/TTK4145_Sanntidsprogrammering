use std::net::UdpSocket;
use std::net::TcpStream;
use std::io::prelude::*;
use std::thread::sleep;
use std::time;

fn main() {
    

    //      UDP sending and reciving
    let socket = UdpSocket::bind("0.0.0.0:20004").expect("couldn't bind to address");

    let message = "Hello World!#¤";
    let addr = "10.100.23.204:20004";
    let mut buf = [0; 1024]; 

    socket.send_to(message.as_bytes(), addr).expect("couldn't send data");
    println!("[SEND]\tSent message: {}", message);


    let mut buf = [0; 1024];
    let (number_of_bytes, src_addr) = socket.recv_from(&mut buf)
                                            .expect("Didn't receive data");
    let _filled_buf = &mut buf[..number_of_bytes];
    println!("[RECV]\tReceived {} bytes from {}", number_of_bytes, src_addr);
    println!("[RECV]\tData: {:?}", _filled_buf);
    println!("[RECV]\tData as string: {:?}", String::from_utf8_lossy(_filled_buf));
    //

    /*      TCP sending and reciving
    let message = "Hello, World!";
    let addr = "10.100.23.204:34933";
    let mut buf = [0; 100];

    let mut stream = TcpStream::connect(addr).expect("could not connect");
    stream.read(&mut buf).expect("could not read");
    //println!("[RECV]\tData: {:?}", buf);
    println!("[RECV]\tData as string: {:?}", String::from_utf8_lossy(&buf));
    //

    let second = time::Duration::from_secs(1);

    let message2 = b"Connect to: 10.100.23.14:20004\0";

    stream.write(b"echo \"Connect to: 10.100.23.14:20004\"\0");

    let mut buf2 = [0; 100];
    loop {
        stream.read(&mut buf2).expect("could not read");
        println!("[RECV]\tData as string: {:?}", String::from_utf8_lossy(&buf2));
        sleep(second);
    }
    //println!("[RECV]\tData: {:?}", buf);
    */
}




