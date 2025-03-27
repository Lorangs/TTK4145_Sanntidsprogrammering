# TTK4145_Sanntidsprogrammering

## Design philosophy
The elevator system is design as a master-slave system with backup. The master handles all the incoming orders and distributes them to
the slaves. The slaves is following orders as long as they are connected to the master, and is kept as simple as possible. The backup only receives data from the master and can create a new master with the data in case the master fails. We start a backup on all computers that are not the master, but the master only connects to one backup, the others are just waiting to connect in case the current backup fails.

The software is designed to use as few external libraryes as possible, exept from the channel library "crossbeam" and the serialisation/deserialisation library "serde". This to ceep the code as simple and meintainable as possible.

We mainly handle synchronization between threads with the crossbeam channel library, but also use some mutexes. 

Contributers: 
- Antonie Barmen
- Lorang Strand
- William Fosser
