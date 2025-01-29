enum ElevatorBehaviour {
    Idle,
    DoorOpen,
    Moving,
}

fn eb_toString(eb:ElevatorBehaviour )->char*{
    return
        eb == EB_Idle       ? "EB_Idle"         :
        eb == EB_DoorOpen   ? "EB_DoorOpen"     :
        eb == EB_Moving     ? "EB_Moving"       :
                              "EB_UNDEFINED"    ;
}
impl ElevatorBehaviour {
    fn to_string(&self) -> &'static str {
        match self {
            ElevatorBehaviour::Idle => "EB_Idle",
            ElevatorBehaviour::DoorOpen => "EB_DoorOpen",
            ElevatorBehaviour::Moving => "EB_Moving",
        }
    }
}


fn elevator_print(es:Elevator){
    println!("  +--------------------+
            \n  |floor = {} |
            \n  |dirn  = {} |
            \n  |behav = {} |",
             es.floor,
             elevio_dirn_toString(es.dirn),
             eb_toString(es.behaviour)
    );
    println!("  +--------------------+\n");
    println!("  |  | up  | dn  | cab |\n");
    for(int floor = N_FLOORS-1; floor >= 0; floor--){
        println!("  | {}", floor);
        for(int btn = 0; btn < N_BUTTONS; btn++){
            if((floor == N_FLOORS-1 && btn == B_HallUp)  || 
               (floor == 0 && btn == B_HallDown) 
            ){
                println!("|     ");
            } else {
                println!(es.requests[floor][btn] ? "|  #  " : "|  -  ");
            }
        }
        println!("|\n");
    }
    println!("  +--------------------+\n");
}

fn elevator_uninitialized()->Elevator{
    return (Elevator){
        .floor = -1,
        .dirn = D_Stop,
        .behaviour = EB_Idle,
        .config = {
            .clearRequestVariant = CV_All,
            .doorOpenDuration_s = 3.0,
        },
    };
}
