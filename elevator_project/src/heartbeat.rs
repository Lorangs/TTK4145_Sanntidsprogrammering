use std::{process, thread};
use crossbeam_channel as cbc;
use network_rust::udpnet::{self, peers::PeerUpdate};


pub fn send_alive(id: String, heartbeat_port: u16){
    //Sender for peer discovery
    let (heartbeat_enable_tx,heartbeat_enable_rx) = cbc::unbounded::<bool>();
    {
        thread::spawn(move || {
            if let Err(_) = udpnet::peers::tx(heartbeat_port, id, heartbeat_enable_rx) {
                process::exit(1);
            }
        });
        thread::spawn(move || {
            // Can stop sending by setting this to false
            heartbeat_enable_tx.send(true).expect("peer_tx_enable_tx"); 
            loop {}
        });
    }
}

pub fn recieve_online_status(heartbeat_update_tx: cbc::Sender<PeerUpdate>,heartbeat_listen_port: u16){
    {
        thread::spawn(move || {
            if udpnet::peers::rx(heartbeat_listen_port, heartbeat_update_tx).is_err() {
                process::exit(1);
            }
        });
    }
}

