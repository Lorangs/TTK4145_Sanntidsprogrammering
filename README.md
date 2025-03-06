# TTK4145_Sanntidsprogrammering_snapshot_hand_in

## Design philosophy
The elevator system is design as a master-slave system with backup. The master handles all the incoming orders and distributes them to
the slaves. The slaves is following orders as long as they are connected to the master, and is kept as simple as possible. The backup is a copy of the part of the master that handles the orders, without the part that communicate with the slaves. A new master is created with tha data from the backup in case the master fails. 

The software is designed to use as few exxternal libraryes as possible, exept from the channel library crossbeam and the serialisation/deserialisation library serde. This to ceep the code as simple and meintainable as possible.

We mainly handle synchronization between threads with the crossbeam channel library, but also use some mutexes. 

### Things we want feedback on: 
#### The big picture:
- The overall structure of the code
- Module design
- Code readability
- Rusty-ness of the code (we are all new to rust)


#### The elevator logic:



#### The network code:



#### Other things:




### Things we know we need to fix/improve: 
- Erro handlig. (apreciate tips on how)
- Refining network code to be more robust, setting tcp timouts, blocking/non-blocking sockets, set_nodelay() etc.
- Handlign IP-adresses and ports in a more elegant way. Using rusts build in IP-adress and port types.
- Refactor the code to be more modular. (Probebly increses readability too)
- Refactor the code to be more idiomatic rust. (do rust programmers use for loops?)
- Add small delay to all infinite loops to prevent 100% CPU usage.

- Local operating mode in cases where a elevator disconeects from the network is not implemented. 