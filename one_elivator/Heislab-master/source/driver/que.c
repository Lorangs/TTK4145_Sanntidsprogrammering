#include "elevio.h"
#include "que.h"


int floorpanel[6][2]={{-1,-1},{-1,-1},{-1,-1},{-1,-1},{-1,-1},{-1,-1}};  //matrix med [etasje nummer, opp=1 ned =0]

int elevatorpanel[4]={-1,-1,-1,-1};  //matrix med [etasje nummer]


void addfloor(int level, int direction){
    int already_exsist=0;
    for (int i=0; i < 6; i++){
        if (floorpanel[i][0]==level && (floorpanel[i][1]==direction)){
            already_exsist=1;
        }
    }
    if (!already_exsist){
        for (int i=0; i < 6; i++){
            if (floorpanel[i][0]==-1){
                floorpanel[i][0]=level;
                floorpanel[i][1]=direction;
                if(direction==1){
                    elevio_buttonLamp(level,BUTTON_HALL_UP,1);
                }
                else{
                    elevio_buttonLamp(level,BUTTON_HALL_DOWN,1);
                }
                break;
            }
        }
    }
}

void addelevator(int level){
    int already_exsist=0;
    for (int i=0; i < 4; i++){
        if (elevatorpanel[i]==level){
            already_exsist=1;
        }
    }
    if (!already_exsist){
        for (int i=0; i < 4; i++){
            if (elevatorpanel[i]==-1){
                elevatorpanel[i]=level;
                elevio_buttonLamp(level,BUTTON_CAB,1);
                break;
            }
        }
    }
}

void check_if_buttons_pressed(void){
    for (int i=0; i < 4; i++){
        if (elevio_callButton(i,BUTTON_HALL_DOWN)){
            addfloor(i,0);
        }
        if (elevio_callButton(i,BUTTON_HALL_UP)){
            addfloor(i,1);
        }
        if (elevio_callButton(i,BUTTON_CAB)){
            addelevator(i);
        }
    }
}

void elevator_order_executed(int level){
    for (int i=0; i < 4; i++){
        if (elevatorpanel[i]==level){
            elevatorpanel[i]=-1;
            elevio_buttonLamp(level,BUTTON_CAB,0);
            break;
        }
    }
    for (int i=0; i < 3; i++){
        if (elevatorpanel[i]==-1){
            elevatorpanel[i]=elevatorpanel[i+1];
            elevatorpanel[i+1]=-1;
        }
    }
}

void floor_order_executed(int level, int direction){
    for (int i=0; i < 6; i++){
        if (floorpanel[i][0]==level && (floorpanel[i][1]==direction)){
            floorpanel[i][0]=-1;
            floorpanel[i][1]=-1;
            if(direction==1){
                elevio_buttonLamp(level,BUTTON_HALL_UP,0);
            }
            else{
                elevio_buttonLamp(level,BUTTON_HALL_DOWN,0);
            }
            break;
        }
    }
    for (int i=0; i < 5; i++){
        if (floorpanel[i][0]==-1){
            floorpanel[i][0]=floorpanel[i+1][0];
            floorpanel[i][1]=floorpanel[i+1][1];
            floorpanel[i+1][0]=-1;
            floorpanel[i+1][1]=-1;
        }
    }
}

int find_order_to_execute(void){
    if(!(elevatorpanel[0]==-1)){
        return elevatorpanel[0];                
    }
    else if (!(floorpanel[0][0]==-1)) {
        return floorpanel[0][0];               
    }
    else{
        return -1;               
    }
}

int look_for_order_on_the_way(int current_floor, int end_floor){
    int stop_at=-1;
    int direction=0;
    if(end_floor-current_floor>0){
        direction=1;
    }
    if(direction==1){
        for (int i=current_floor; i < end_floor; i++){
            for (int j=0; j < 4; j++){
                if(elevatorpanel[j]==i){
                    stop_at=i;
                    return stop_at;
                }
            }
            for (int j=0; j < 6; j++){
                if(floorpanel[j][0]==i && (floorpanel[j][1]==direction)){
                    stop_at=i;
                    return stop_at;
                }
            }
        }
    }
    else{               
        for (int i=current_floor; i > end_floor; i--){
            for (int j=0; j < 4; j++){
                if(elevatorpanel[j]==i){
                    stop_at=i;
                    return stop_at;
                }
            }
            for (int j=0; j < 6; j++){
                if(floorpanel[j][0]==i && (floorpanel[j][1]==direction)){
                    stop_at=i;
                    return stop_at;
                }
            }
        }
    }
    return stop_at;
}


void clear_que(){
    for (int i=0; i < 4; i++){
        elevatorpanel[i]=-1;
        elevio_buttonLamp(i,BUTTON_CAB,0);
        elevio_buttonLamp(i,BUTTON_HALL_DOWN,0);
        elevio_buttonLamp(i,BUTTON_HALL_UP,0);
    }
    for (int i=0; i < 6; i++){
        floorpanel[i][0]=-1;
        floorpanel[i][1]=-1;
    }

}