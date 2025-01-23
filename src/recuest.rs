

//må skjekke om for loopane har rett range, kan ver det skal ver eks:N_FLOORS-1 
fn requests_above(e: Elevator)->i32{
    for f:i32 in (e.floor+1)..= N_FLOORS {
        for btn:i32 in 0..=N_BUTTONS{
            if e.requests[f][btn] {
                return 1;
            }
        }
    }
    return 0;
}

fn requests_below(e: Elevator)->i32{
    for f:i32 in 0..= e.floor {
        for btn:i32 in 0..=N_BUTTONS{
            if e.requests[f][btn] {
                return 1;
            }
        }
    }
    return 0;
}

fn requests_here(e: Elevator)->i32{
    for btn:i32 in 0..=N_BUTTONS{
        if e.requests[f][btn] {
            return 1;
        }
    }
    return 0;
}


fn requests_chooseDirection(e: Elevator) ->DirnBehaviourPair{
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



fn requests_shouldStop(e: Elevator)->i32{
    match e.dirn{
    D_Down=>
        e.requests[e.floor][B_HallDown] ||
        e.requests[e.floor][B_Cab]      ||
        !requests_below(e);
    D_Up => 
        e.requests[e.floor][B_HallUp]   ||
        e.requests[e.floor][B_Cab]      ||
        !requests_above(e);
    D_Stop=>
    _=>1;
    }
}

fn requests_shouldClearImmediately(e: Elevator, btn_floor:i32, btn_type:Button)->i32{
    match e.config.clearRequestVariant{
    CV_All=>    e.floor == btn_floor;
    CV_InDirn=> e.floor == btn_floor && 
            (
                (e.dirn == D_Up   && btn_type == B_HallUp)    ||
                (e.dirn == D_Down && btn_type == B_HallDown)  ||
                e.dirn == D_Stop ||
                btn_type == B_Cab
            );  
    _=> 0;
    }
}

fn requests_clearAtCurrentFloor(e: Elevator)->Elevator{
        
    match e.config.clearRequestVariant{
    CV_All=> 
        for btn:Button in 0..= N_BUTTONS {
            e.requests[e.floor][btn] = 0;
        }
        break;
        
    CV_InDirn=>
        e.requests[e.floor][B_Cab] = 0;
        match e.dirn{
        D_Up=>
            if !requests_above(e) && !e.requests[e.floor][B_HallUp] {
                e.requests[e.floor][B_HallDown] = 0;
            }
            e.requests[e.floor][B_HallUp] = 0;
            break;
            
        D_Down=>
            if(!requests_below(e) && !e.requests[e.floor][B_HallDown]){
                e.requests[e.floor][B_HallUp] = 0;
            }
            e.requests[e.floor][B_HallDown] = 0;
            break;
            
        D_Stop=>
        _=>
            e.requests[e.floor][B_HallUp] = 0;
            e.requests[e.floor][B_HallDown] = 0;
            break;
        }
        break;
        
    _=>
        break;
    }
    
    return e;
}