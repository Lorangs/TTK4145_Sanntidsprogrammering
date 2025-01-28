use driver_rust::elevio::*;
use driver_rust::elevio::elev as e;

//må skjekke om for loopane har rett range, kan ver det skal ver eks:N_FLOORS-1 
fn requests_above(e: Elevator) -> bool{
    for floor:i32 in (e.floor+1).. = N_FLOORS {
        for button:i32 in 0.. = N_BUTTONS{
            if e.requests[floor][button] {
                return true
            }
        }
    }
    false
}

fn requests_below(e: Elevator) -> bool{
    for floor:i32 in 0..= e.floor {
        for button:i32 in 0.. = N_BUTTONS{
            if e.requests[floor][button] {
                return true
            }
        }
    }
    false
}

fn requests_here(e: Elevator) -> bool{
    for button:i32 in 0.. = N_BUTTONS{
        if e.requests[floor][button] {
            return true
        }
    }
    false
}


fn requests_chooseDirection(e: Elevator) -> DirnBehaviourPair{
    match e.dirn{
    D_Up =>  requests_above(e) ? (DirnBehaviourPair){D_Up,   EB_Moving}   :
             requests_here(e)  ? (DirnBehaviourPair){D_Down, EB_DoorOpen} :
             requests_below(e) ? (DirnBehaviourPair){D_Down, EB_Moving}   :
                                 (DirnBehaviourPair){D_Stop, EB_Idle}     ;
    D_Down=> requests_below(e) ? (DirnBehaviourPair){D_Down, EB_Moving}   :
             requests_here(e)  ? (DirnBehaviourPair){D_Up,   EB_DoorOpen} :
             requests_above(e) ? (DirnBehaviourPair){D_Up,   EB_Moving}   :
                                 (DirnBehaviourPair){D_Stop, EB_Idle}     ;
    D_Stop=> requests_here(e)  ? (DirnBehaviourPair){D_Stop, EB_DoorOpen} :
             requests_above(e) ? (DirnBehaviourPair){D_Up,   EB_Moving}   :
             requests_below(e) ? (DirnBehaviourPair){D_Down, EB_Moving}   :
                                 (DirnBehaviourPair){D_Stop, EB_Idle}     ;
    _=> (DirnBehaviourPair){D_Stop, EB_Idle};
    }
}



fn requests_shouldStop(e: Elevator) -> bool{
    match e.dirn{
        D_Down =>
            e.requests[e.floor][B_HallDown] ||
            e.requests[e.floor][B_Cab]      ||
            !requests_below(e)
            
        D_Up => 
            e.requests[e.floor][B_HallUp]   ||
            e.requests[e.floor][B_Cab]      ||
            !requests_above(e)

        // D_Stop =>
        _=> true
    }
}

fn requests_shouldClearImmediately(e: Elevator, btn_floor:u8, btn_type:Button)-> bool{
    match e.config.clearRequestVariant{

        CV_All      =>  e.floor == btn_floor;
        CV_InDirn   =>  e.floor == btn_floor && 
                        (
                            (e.dirn == D_Up   && btn_type == B_HallUp)    ||
                            (e.dirn == D_Down && btn_type == B_HallDown)  ||
                            e.dirn == D_Stop ||
                            btn_type == B_Cab
                        );  
                
        _=> false
    }
}

fn requests_clearAtCurrentFloor(e: Elevator) -> Elevator{
        
    match e.config.clearRequestVariant{
        CV_All => 
            for button:Button in 0.. = N_BUTTONS {
                e.requests[e.floor][button] = false;
            }
            break;
            
        CV_InDirn =>
            e.requests[e.floor][B_Cab] = false;
            match e.dirn{
                D_Up =>
                    if !requests_above(e) && !e.requests[e.floor][B_HallUp] {
                        e.requests[e.floor][B_HallDown] = false;
                    }
                    e.requests[e.floor][B_HallUp] = false;
                    break;
                    
                D_Down =>
                    if(!requests_below(e) && !e.requests[e.floor][B_HallDown]){
                        e.requests[e.floor][B_HallUp] = false;
                    }
                    e.requests[e.floor][B_HallDown] = false;
                    break;
                    
                //D_Stop =>

                _=>
                    e.requests[e.floor][B_HallUp] = false;
                    e.requests[e.floor][B_HallDown] = false;
                    break;
                }
                break;
            
        _=>
            break;
    }
    e
}