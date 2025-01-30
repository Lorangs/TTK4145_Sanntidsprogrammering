// #include <stdio.h>
// #include <stdlib.h>
// #include <unistd.h>

// #include "con_load.h"
// #include "elevator_io_device.h"
// #include "fsm.h"
// #include "timer.h"

use crossbeam_channel as cbc;

mod inputs;
mod timer;
mod elevator;

use crate::elevator::{ElevatorBehaviour};

use driver_rust::elevio::elev as e;

fn main(){
    
    let mut slave = elevator::Slave::init("localhost:15657".to_string()).unwrap();

    // TODO kan kun implemeteres med std::fmt::Display. Dette må konstrueres for Slave
    // println!("Slave initialized:\n{}", slave);

    let channels: inputs::Channels = inputs::get_channels(&slave);
    
    loop {
      cbc::select! {
        recv(channels.call_button_tx_rx.1) -> msg => {
            let call_button = msg.unwrap();
            println!("Received call button message: {:#?}", call_button);
            slave.elevator.call_button_light(call_button.floor, call_button.call, true);
            
            match call_button.call {
                0 => slave.orders[call_button.floor as usize].hall_up = true,
                1 => slave.orders[call_button.floor as usize].hall_down = true,
                2 => slave.orders[call_button.floor as usize].cab_call = true,
                _ => panic!("Mottok ukjent knappetype"),
            }
            match slave.behaviour {
                ElevatorBehaviour::Idle => slave.start_moving(),
                _ => {},
            }
        }

        recv(channels.floor_sensor_tx_rx.1) -> msg => {
            let floor_sensor = msg.unwrap();
            println!("Received floor sensor message: {:#?}", floor_sensor);
            slave.floor = floor_sensor as usize;

            match slave.behaviour {
                ElevatorBehaviour::Moving => {
                    if slave.should_stop() {
                        println!("Stopping at floor {:?}", slave.floor);
                        slave.elevator.motor_direction(e::DIRN_STOP);
                        slave.elevator.door_light(true);
                        slave.behaviour = ElevatorBehaviour::DoorOpen;
                        slave.clear_at_current_floor();

                        slave.door_timer.reset();   // TODO kontroller denne funksjonaliteten. tilbakestiller door_timer til 3 sekunder
                    }
                },
                _ => {},
            }
        }

        recv(channels.stop_button_tx_rx.1) -> msg => {
            let stop_button = msg.unwrap();
            println!("Stop button: {:#?}", stop_button);
            
            slave.behaviour = ElevatorBehaviour::OutOfOrder; 
        }

        recv(channels.obstruction_tx_rx.1) -> msg => {
            let obstr = msg.unwrap();
            println!("Obstruction: {:#?}", obstr);

            match slave.behaviour {
                ElevatorBehaviour::DoorOpen => slave.door_timer.reset(),   // Ikke lukk døren
                _ => {},
            }
        }

        /*
        TODO Denne delen av koden må løses

        recv(channels.door_timer_tx_rx.1) -> msg => {
            println!("Door closing!");
            slave.start_moving(&channels.door_timer_tx_rx);
        }
        */

      }
    }
}


