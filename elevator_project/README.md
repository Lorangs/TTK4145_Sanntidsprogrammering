# TTK4145_Sanntidsprogrammering

## Prerequisites
This program depends on the hall_request_assigner program, which is released as source code in the TTK4145 course.
hall_request_assigner is an optimization algorithm that assigns orders to elevators. This program must be compiled before starting by running:
./hall_request_assigner/build.sh

Remember to allow execution permissions for build.sh by running: (chmod +x hall_request_assigner/build.sh)

It is also important to change the static IP addresses in the config.json file to match the computers you plan to run the program on.


## Design philosophy
The elevator system is design as a master-slave system with backup. The master handles all the incoming orders and distributes them to
the slaves. The slaves is following orders as long as they are connected to the master, and is kept as simple as possible. The backup only receives data from the master and can create a new master with the data in case the master fails. We also start a backup on all computers that are not the master, but the master only connects to one backup, the others are just waiting to connect in case the current backup fails.

## Software Design
The software is designed to use as few external libraries as possible, except for:
- Crossbeam – A channel library for efficient inter-thread communication.
- Serde – A serialization/deserialization library.
- Some additional provided libraries.
This to ceep the code as simple and meintainable as possible.

## Synchronization
We mainly handle synchronization between threads with the crossbeam channel library, but also use some mutexes. 

## Network Communication
- To detect network loss, we use the network-rust library, which employs UDP for fast error detection.
- Otherwise, TCP is used for all message passing.

