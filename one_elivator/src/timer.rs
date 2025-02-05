
use std::time::Duration;
use crossbeam_channel as cbc;
use std::thread::{spawn, sleep};


pub fn start_timer (channel: &cbc::Sender<()>, duration: Duration) {
    let tx = channel.clone();
    spawn(move || {
        sleep(duration);
        let _ =  tx.send(()).unwrap();
    });
}

