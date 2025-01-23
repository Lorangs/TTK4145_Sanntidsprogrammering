// #include <stdio.h>
// #include <stdlib.h>
// #include <unistd.h>

// #include "con_load.h"
// #include "elevator_io_device.h"
// #include "fsm.h"
// #include "timer.h"
use tokio::time::{sleep, Duration};
mod request;
mod timer;


fn main(void){
    println!("Started!\n");
    
    let inputPollRate_ms:i32 = 25;
    con_load("elevator.con",
        con_val("inputPollRate_ms", &inputPollRate_ms, "%d")
    )
    
    let input:ElevInputDevice = elevio_getInputDevice();    
    
    if input.floorSensor()==-1 {
        fsm_onInitBetweenFloors();
    }
        
    loop{
        { // Request button
            static mut prev: [[i32; N_BUTTONS]; N_FLOORS] = [[0; N_BUTTONS]; N_FLOORS]; //orginal: static int prev[N_FLOORS][N_BUTTONS]; ??
            for f:i32 in 0..=N_FLOORS {
                for b:i32 in 0..=N_BUTTONS {
                    let v:i32 = input.requestButton(f, b);
                    if v  &&  v != prev[f][b] {
                        fsm_onRequestButtonPress(f, b);
                    }
                    prev[f][b] = v;
                }
            }
        }
        
        { // Floor sensor
            static prev:i32 = -1;
            let f:i32 = input.floorSensor();
            if f != -1  &&  f != prev {
                fsm_onFloorArrival(f);
            }
            prev = f;
        }
        
        
        { // Timer
            if timer::timer_timedOut() {
                timer_stop();
                fsm_onDoorTimeout();
            }
        }
        
        sleep(Duration::from_millis(1000)).await;
    }
}