use std::{process, thread};
use crossbeam_channel as cbc;
use network_rust::udpnet::{self, peers::PeerUpdate};

pub fn send_alive(elevator_number: u8, peer_send_port: u16){
    let peer_port = peer_send_port;//19738;
    //Sender for peer discovery
    let (peer_tx_enable_tx,peer_tx_enable_rx) = cbc::unbounded::<bool>();
    {
        thread::spawn(move || {
            if udpnet::peers::tx(peer_port, elevator_number.to_string(), peer_tx_enable_rx).is_err() {
                // crash program if creating the socket fails (peers:tx will always block if the
                // initialization succeeds)
                process::exit(1);
            }
        });
        thread::spawn(move || {
            // Can stop sending by setting this to false
            peer_tx_enable_tx.send(true).expect("peer_tx_enable_tx"); 
            loop {}
        });
    }
}

pub fn recieve_online_statuses(peer_update_tx: cbc::Sender<PeerUpdate>,peer_listen_port: u16){
    let peer_port = peer_listen_port;//19738;
    {
        thread::spawn(move || {
            if udpnet::peers::rx(peer_port, peer_update_tx).is_err() {
                // crash program if creating the socket fails (peers:rx will always block if the
                // initialization succeeds)
                process::exit(1);
            }
        });
    }
}
Tuva
Tuva Bjørbekk
/// Sends connectivity upates of other controllers on returned channel
fn initiate_peer_updates(elevator_id: u8, peer_send_port:u16 ,peer_listen_port:u16 ) -> cbc::Receiver<udpnet::peers::PeerUpdate>{
    connectivity::send_alive(elevator_id,peer_send_port);
    let (peer_update_tx, peer_update_rx) = cbc::unbounded::<udpnet::peers::PeerUpdate>();
    connectivity::recieve_online_statuses(peer_update_tx, peer_listen_port);
    peer_update_rx
}
Tuva
Tuva Bjørbekk
    /// Sets new peers to active and lost peers as inactive in the controllers list
    fn set_active_elevators(&mut self, peer_update: &PeerUpdate){
        if let Some(new_peer_str) = &peer_update.new{
            if let Ok(new_peer) = new_peer_str.parse::<usize>(){
                self.active_elevators[new_peer] = true;
            }
        }
        for peer in &peer_update.lost {
            if let Ok(peer_index) = peer.parse::<usize>() {
                if peer_index != self.elevator_number as usize{
                    self.active_elevators[peer_index] = false;
                }
            }
        }
    }
