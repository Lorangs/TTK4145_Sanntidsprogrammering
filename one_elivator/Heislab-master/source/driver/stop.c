#include <assert.h>
#include <stdlib.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netdb.h>
#include <stdio.h>
#include <pthread.h>

#include "elevio.h"
#include "con_load.h"
#include "door.h"
#include "stop.h"
#include "que.h"

int door_open;

void stop(){
    while(elevio_stopButton()){
        elevio_motorDirection(DIRN_STOP);
        elevio_stopLamp(1);
        clear_que();
        clear_end_and_stop_at_floors();
        if(elevio_floorSensor()!=-1){
            open_door_after_stop();
            door_open=1;
        }
    }
    elevio_stopLamp(0);
    if(door_open){
        if(close_door()){
            door_open=0;
        }
    }
}