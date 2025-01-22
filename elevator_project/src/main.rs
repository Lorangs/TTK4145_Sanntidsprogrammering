use driver_rust::elevio;
use driver_rust::elevio::elev as e;

fn main() {
    let elev_num_floors = 4;
    let elevator = e::Elevator::init("localhost:15657", elev_num_floors);
}