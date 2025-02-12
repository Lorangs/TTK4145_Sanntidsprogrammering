mod inputs;
mod timer;
mod elevator;
use elevator::{ElevatorBehaviour, Dirn, Slave};

use crossbeam_channel as cbc;
use std::time::Duration;
use driver_rust::elevio::elev as e;

fn main(){

    
    let mut slave = Slave::init("localhost:15657".to_string()).unwrap();
    println!("Slave initialized");

    let channels: inputs::Channels = inputs::get_channels(&slave);
    println!("Channels initialized");
    
    let (timer_tx, timer_rx) = cbc::unbounded::<bool>();
    println!("Timer initialized");



    // går til neders etasje ved initialisering
    // slave.behaviour = ElevatorBehaviour::Moving;
    // slave.direction = Dirn::Down;
    // slave.elevator.motor_direction(e::DIRN_DOWN);
    slave.sync_lights();
    slave.elevator.door_light(false);

    slave.behaviour = ElevatorBehaviour::Moving;
    slave.direction = Dirn::Down;
    slave.elevator.motor_direction(e::DIRN_DOWN);
    
    
    loop {
        
        cbc::select! {
            recv(channels.floor_sensor_rx) -> msg => {
                let floor_sensor = msg.unwrap();
                println!("Received floor sensor message: {:#?}", floor_sensor);
                slave.floor = floor_sensor as usize;
                if slave.floor !=usize::MAX{
                    slave.elevator.motor_direction(e::DIRN_STOP);
                    slave.direction = Dirn::Stop;
                    slave.behaviour = ElevatorBehaviour::Idle;
                    slave.elevator.floor_indicator(slave.floor as u8);
                    break;
                }
            }
        }
    }
    
    println!("Slave Ready:\n{:#?}", slave);

    loop {
      cbc::select! {
        // Receive call button message
        recv(channels.call_button_rx) -> msg => {
            let call_button = msg.unwrap();
            println!("Received call button message: {:#?}", call_button);

            match call_button.call {
                0 => slave.orders[call_button.floor as usize].hall_up = true,
                1 => slave.orders[call_button.floor as usize].hall_down = true,
                2 => slave.orders[call_button.floor as usize].cab_call = true,
                _ => panic!("Mottok ukjent knappetype"),
            }

            slave.sync_lights();
            
            match slave.behaviour {
                ElevatorBehaviour::Idle => {
                    slave.start_moving(&timer_tx);
                },
                _ => {},
            }
        }
        
        // Receive floor sensor message
        recv(channels.floor_sensor_rx) -> msg => {
            let floor_sensor = msg.unwrap();
            println!("Received floor sensor message: {:#?}", floor_sensor);
            slave.floor = floor_sensor as usize;

            match slave.behaviour {
                ElevatorBehaviour::Moving => { slave.floor = floor_sensor as usize;
                    slave.elevator.floor_indicator(slave.floor as u8);
                    if slave.should_stop() {
                        println!("Stopping at floor {:?}", slave.floor);
                        slave.behaviour = ElevatorBehaviour::DoorOpen;
                        slave.elevator.door_light(true);
                        slave.clear_at_current_floor();
                        slave.sync_lights();
                        slave.elevator.motor_direction(e::DIRN_STOP);

                        timer::start_timer(&timer_tx, Duration::from_secs(3));    // starting doortimer
                    }
                },
                _ => {},
            }
        }

        // Receive stop button message
        recv(channels.stop_button_rx) -> msg => {
            let stop_button = msg.unwrap();
            println!("Stop button: {:#?}", stop_button);
            slave.elevator.motor_direction(e::DIRN_STOP);
            slave.behaviour = ElevatorBehaviour::OutOfOrder; 
        }

        recv(channels.obstruction_rx) -> msg => {
            let obstr = msg.unwrap();
            slave.obstruction = obstr;

            println!("Obstruction: {:#?}", obstr);
        }

        // Receive timer message
        recv(timer_rx) -> _msg => {
            if slave.obstruction {
                //println!("Obstruction detected. Timer reset.");
                timer::start_timer(&timer_tx, Duration::from_secs(3));
            }
            else {
                println!("Timer expired. Door closing.");
                slave.elevator.door_light(false);
                slave.start_moving(&timer_tx);
            }
        }
    }
    }
}




//Notater: Lamper fungerer ikke. Opp og ned lampe slukkes samtidig (Skal bare være en om gangen)