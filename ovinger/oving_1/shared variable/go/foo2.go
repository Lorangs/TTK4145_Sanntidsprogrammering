// Use `go run foo.go` to run your program

package main

import (
    . "fmt"
    "runtime"
)

var i = 0

func incrementing(done chan bool) {
    //TODO: increment i 1000000 times
    for j := 0; j < 1000000; j++ {
        i++
    }
	done <- true
}

func decrementing(done chan bool) {
    //TODO: decrement i 1000000 times
    for j := 0; j < 1000000; j++ {
        i--
    }
	done <- true
}

func main() {
    // What does GOMAXPROCS do? What happens if you set it to 1?
    runtime.GOMAXPROCS(2)    
	// two threads to run the two functions concurrently

	done := make(chan bool, 2)

    // TODO: Spawn both functions as goroutines
    go incrementing(done)
    go decrementing(done)

	for k:= 0; k < 2; k++{
		select {
			case <- done:
		}
	}
    // We have no direct way to wait for the completion of a goroutine (without additional synchronization of some sort)
    // We will do it properly with channels soon. For now: Sleep.
    Println("The magic number is:", i)
}
