use crossbeam_channel as cbc;
use std::thread::spawn;
use driver_rust::elevio::{self};

use crate::elevator;

#[derive(Debug, Clone)]
pub struct Channels {
    //pub floor_sensor_tx:    cbc::Sender<u8>,          Behøves tx her?
    pub floor_sensor_rx:    cbc::Receiver<u8>,
    //pub call_button_tx :    cbc::Sender<elevio::poll::CallButton>,
    pub call_button_rx :    cbc::Receiver<elevio::poll::CallButton>,
    //pub stop_button_tx :    cbc::Sender<bool>,
    pub stop_button_rx :    cbc::Receiver<bool>,
    //pub obstruction_tx :    cbc::Sender<bool>, 
    pub obstruction_rx :    cbc::Receiver<bool>,
}

pub fn get_channels (slave: &elevator::Slave) -> Channels {
    let poll_period = slave.config.poll_period_ms;

    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
    {
        let elevator = slave.elevator.clone();
        let call_tx  = call_button_tx.clone();
        spawn(move || elevio::poll::call_buttons(elevator, call_tx, poll_period));
    }

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
    {
        let elevator = slave.elevator.clone();
        let floor_tx  = floor_sensor_tx.clone();
        spawn(move || elevio::poll::floor_sensor(elevator, floor_tx, poll_period));
    }

    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
    {
        let elevator = slave.elevator.clone();
        let stop_tx  = stop_button_tx.clone();
        spawn(move || elevio::poll::stop_button(elevator, stop_tx, poll_period));
    }

    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();
    {
        let elevator = slave.elevator.clone();
        let obstr_tx  = obstruction_tx.clone();
        spawn(move || elevio::poll::obstruction(elevator, obstr_tx, poll_period));
    }

    Channels {
        //floor_sensor_tx,
        floor_sensor_rx,
        //call_button_tx,
        call_button_rx,
        //stop_button_tx,
        stop_button_rx,
        //obstruction_tx,
        obstruction_rx,
    }
}