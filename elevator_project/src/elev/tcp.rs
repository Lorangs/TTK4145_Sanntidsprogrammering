// This file contains the TCP module, which is responsible for handling the TCP connection between the elevator and the scheduler.

enum Message{
    NewOrder(u8),
    OrderComplete(u8),
    Error(ErrorState),
}


enum ErrorState {
    OK,
    NødStopp,
    DørObstruksjon,
    Nettverk(std::io::ErrorKind),
}
