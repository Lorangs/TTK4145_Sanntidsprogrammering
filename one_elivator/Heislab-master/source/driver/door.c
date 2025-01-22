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

int timer_startet=0;
int door_open = 0;
double total_t=0;

void open_door_after_stop(){
    elevio_doorOpenLamp(1);
    timer_startet=0; 
}

void open_door(){
    elevio_doorOpenLamp(1);
}

time_t start_t, end_t;
int close_door(){
    int obstruction = elevio_obstruction();
    if (obstruction == 0){
        if(!timer_startet){
            start_t=time(NULL);
            total_t = 0;
            timer_startet=1;
        }
        if (total_t < 3) {
            end_t = time(NULL);
            total_t =end_t-start_t;
        } 
        else {
            timer_startet=0;
            elevio_doorOpenLamp(0);
            return 1;
        }
    }
    else{
        timer_startet=0;
    }
    return 0; 
}
