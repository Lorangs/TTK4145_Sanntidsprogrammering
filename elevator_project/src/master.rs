

use crate::elev::config::Config;

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub hall_down   : bool,
    pub hall_up     : bool,
    pub cab_call    : bool,
}

[derive(Debug, Clone)]
struct Master {
    pub master_ip   :         String,                                                   // IP address of master
    pub backup_ip   :         String,                                                   // IP address of backup
    pub slaves_ip   :         [String, NUMBER_OF_ELEVATORS ]                            // Vector of slaves IP addresses
    pub slaves_order:         [[Order; NUMBER_OF_FLOORS]; NUMBER_OF_ELEVATORS]          // Vector of slaves order queues

    
}


impl Master {
    pub fn init(config: Config) -> Result<Master> {
        Ok( Self  {
            master_ip       : config.elevator_ip_list[0],                               // IP address of master
            backup_ip       : config.elevator_ip_list[1],                               // IP address of backup
            slaves_ip       : config.elevator_ip_list[0..],                             // Vector of slaves IP addresses                 
            slaves_order    : [Order
                                    {
                                        hall_down   : false,
                                        hall_up     : false,
                                        cab_call    : false,    
                                    }; NUMBER_OF_ELEVATORS],
        })
    }
}