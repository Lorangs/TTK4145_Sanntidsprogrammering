// #include <stdio.h>
// #include <stdlib.h>
// #include <unistd.h>

// #include "con_load.h"
// #include "elevator_io_device.h"
// #include "fsm.h"
// #include "timer.h"

use crossbeam_channel as cbc;
use elevator::Dirn;

mod inputs;
mod timer;
mod elevator;

use crate::elevator::ElevatorBehaviour;

use driver_rust::elevio::elev as e;

fn main(){
    
    let mut slave = elevator::Slave::init("localhost:15657".to_string()).unwrap();
    
    let channels: inputs::Channels = inputs::get_channels(&slave);
    
    // TODO kan kun implemeteres med std::fmt::Display. Dette må konstrueres for Slave
    // println!("Slave initialized:\n{}", slave);
    
    // går til neders etasje ved initialisering
    slave.behaviour = ElevatorBehaviour::Moving;
    slave.dirn = Dirn::Down;
    slave.elevator.motor_direction(e::DIRN_DOWN);
    slave.sync_light();
    slave.elevator.door_light(false);
    loop {
        cbc::select! {
            recv(channels.floor_sensor_tx_rx.1) -> msg => {
                let floor_sensor = msg.unwrap();
                println!("Received floor sensor message: {:#?}", floor_sensor);
                slave.floor = floor_sensor as usize;
                if slave.floor == 0 {
                    slave.elevator.motor_direction(e::DIRN_STOP);
                    slave.dirn = Dirn::Stop;
                    slave.behaviour = ElevatorBehaviour::Idle;
                    break;
                }
            }
        }
    }
    
    println!("Slave initialized:\n{:#?}", slave);

    loop {
      cbc::select! {
        recv(channels.call_button_tx_rx.1) -> msg => {
            let call_button = msg.unwrap();
            println!("Received call button message: {:#?}", call_button);

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
            slave.sync_light();
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
                        slave.sync_light();

                        slave.timer.reset();   // TODO kontroller denne funksjonaliteten. tilbakestiller timer til 3 sekunder
                    }
                },
                _ => {},
            }
        }

        recv(channels.stop_button_tx_rx.1) -> msg => {
            let stop_button = msg.unwrap();
            println!("Stop button: {:#?}", stop_button);
            slave.elevator.motor_direction(e::DIRN_STOP);
            slave.behaviour = ElevatorBehaviour::OutOfOrder; 
        }

        recv(channels.obstruction_tx_rx.1) -> msg => {
            let obstr = msg.unwrap();
            slave.obstruction = obstr;

            println!("Obstruction: {:#?}", obstr);
        }

        recv(slave.timer.channel.1) -> _msg => {

            if slave.obstruction {
                println!("Obstruction detected. Timer reset.");
                slave.timer.reset();
            }
            else {
                println!("Timer expired. Door closing.");
                slave.elevator.door_light(false);
                slave.start_moving();
            }
        }
    }
    }
}


