use crossbeam_channel as cbc;
use std::thread::spawn;
use driver_rust::elevio::{self};

use crate::{slave, config};

#[derive(Debug, Clone)]
pub struct Channels {
    pub floor_sensor_rx     : cbc::Receiver<u8>,
    pub call_button_rx      : cbc::Receiver<elevio::poll::CallButton>,
    pub stop_button_rx      : cbc::Receiver<bool>, 
    pub obstruction_rx      : cbc::Receiver<bool>,
}

pub fn get_channels (elevator: &elevio::elev::Elevator, config: config::Config) -> Channels {
    let poll_period = config.input_poll_rate_ms;

    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::call_buttons(elevator, call_button_tx, poll_period));
    }

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period));
    }

    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::stop_button(elevator, stop_button_tx, poll_period));
    }

    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::obstruction(elevator, obstruction_tx, poll_period));
    }
    Channels {
        floor_sensor_rx,
        call_button_rx,
        stop_button_rx,
        obstruction_rx,
    }
}
