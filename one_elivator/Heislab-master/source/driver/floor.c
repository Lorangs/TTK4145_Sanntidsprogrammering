#include <assert.h>
#include <stdlib.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netdb.h>
#include <stdio.h>
#include <pthread.h>

#include "elevio.h"
#include "con_load.h"
#include "floor.h"

int current_floor_state = -1;
/*Hvis heisen starter i udefinert tilstand, skal heisen kjøre opp.*/

int floor_state_and_light(){

    int floor_sensor = elevio_floorSensor();

    if (floor_sensor != -1){
        current_floor_state = floor_sensor;
        elevio_floorIndicator(current_floor_state);
    }
    return current_floor_state;
}



