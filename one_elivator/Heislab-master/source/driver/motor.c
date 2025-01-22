#include "elevio.h"
#include "motor.h"
#include "door.h"


void execute_an_order(int current_floor, int end_floor, int stop_at){
    if (current_floor< end_floor && (end_floor!=-1)){
        elevio_motorDirection(DIRN_UP); 
    }
    if (current_floor> end_floor && (end_floor!=-1)){
        elevio_motorDirection(DIRN_DOWN); 
    }
    if(current_floor == stop_at && (elevio_floorSensor()!=-1)){
        elevio_motorDirection(DIRN_STOP);
        open_door();
        return;
    }
    if (current_floor==end_floor && (elevio_floorSensor()!=-1)) {
        elevio_motorDirection(DIRN_STOP);
        open_door();
        return;
    }
}