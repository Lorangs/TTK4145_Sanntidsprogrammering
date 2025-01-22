#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <time.h>
#include "driver/elevio.h"
#include "driver/que.h"
#include "driver/door.h"
#include "driver/floor.h"
#include "driver/stop.h"


int end_floor=-1;
int stop_at=-1;

int main(){
    elevio_init();

    check_if_buttons_pressed();

    while(elevio_floorSensor()==-1){
        elevio_motorDirection(DIRN_UP);
    }
    elevio_motorDirection(DIRN_STOP);
    

    while(1){        

        int current_floor=floor_state_and_light();
        
        if(end_floor==-1 && elevio_floorSensor!=-1){
            elevio_motorDirection(DIRN_STOP);
        }

        stop_at=look_for_order_on_the_way(current_floor,end_floor);

        execute_an_order(current_floor,end_floor,stop_at);
        
        if(end_floor==-1){
            end_floor=find_order_to_execute();
        }

        if(end_floor==current_floor && (elevio_floorSensor()!=-1)){
            if (close_door()==1){
                floor_order_executed(end_floor,1);
                floor_order_executed(end_floor,0);
                elevator_order_executed(end_floor);
                end_floor=find_order_to_execute();
            }
        }
        if(stop_at==current_floor && (elevio_floorSensor()!=-1)){
            if (close_door()==1){
                elevator_order_executed(stop_at);
                floor_order_executed(stop_at,0);
                floor_order_executed(stop_at,1);
            }
        }
        
        check_if_buttons_pressed();
        stop();

        if(elevio_stopButton() && elevio_callButton(0, BUTTON_CAB)){
            clear_que();
            elevio_doorOpenLamp(0);
            break;
        }
        
        nanosleep(&(struct timespec){0, 20*1000*1000}, NULL);  
    }
    return 0;
}

void clear_end_and_stop_at_floors(){
    end_floor=-1;
    stop_at=-1;
}