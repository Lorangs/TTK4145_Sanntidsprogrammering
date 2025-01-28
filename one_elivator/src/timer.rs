use std::time::Instant;

static fn get_wall_time() ->f32{
    let now = Instant::now();
    return now;
}

//usiker på ka eg ska gjer med static
static  timerEndTime:  f32;
static  timerActive:   i32;

fn timer_start(duration: f32){
    timerEndTime    = get_wall_time() + duration;
    timerActive     = 1;
}

fn timer_stop(){
    timerActive = 0;
}

fn timer_timedOut()->i32{
    return (timerActive  &&  (get_wall_time() > timerEndTime));
}