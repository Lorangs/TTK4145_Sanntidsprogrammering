mod config;
mod test;
mod slave;
mod master;
mod tcp;


use driver_rust::elevio;
use driver_rust::elevio::elev as e;



fn main() -> std::result::Result<(), std::io::Error> {

    test::test_Config();

    let path = Path::new("config.json");
    let config = config::config(&path)?;
    println!("[MAIN]\t\tStarting elevator driver");


    let elevator = match e::Elevator::init(
        &config::get_full_ip_address(&config.elevator_ip_list[0], config.master_port), 
        config.number_of_floors) 
        {
            Ok(elevator) => elevator,
            Err(error) => match error.kind() {
                ErrorKind::ConnectionRefused => panic!("[MAIN]\t\tFailed to connect to elevator: {}", error),
                ErrorKind::AddrNotAvailable => panic!("[MAIN]\t\tInvalid IP address: {}", error),
                other_error => panic!("[MAIN]\t\tUnexpected error: {}", other_error),
        },
    };
    println!("[MAIN]\tElevator started:\n{:#?}", elevator);


}