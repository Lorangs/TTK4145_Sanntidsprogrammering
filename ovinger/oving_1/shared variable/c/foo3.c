// Compile with `gcc foo.c -Wall -std=gnu99 -lpthread`, or use the makefile
// The executable will be named `foo` if you use the makefile, or `a.out` if you use gcc directly

// test med semaphores.
// både semaphorer og mutexes gir samme resultat. Begge gjør prosesser interaktivt på samme tråd. Usikker på hvor forskjellen ligger.

#include <pthread.h>
#include <semaphore.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>

int i = 0;
    pthread_t thread1;
    pthread_t thread2;
    sem_t sem;

// Note the return type: void*
void* incrementingThreadFunction(){
    // TODO: increment i 1_000_000 timesmak
    for (int j = 0; j < 10; j++) {
        sem_wait(&sem);
        i++;
        printf("Incrementing: %d\n", i);
        sem_post(&sem);
        sleep(1);
    }
    return NULL;
}

void* decrementingThreadFunction(){
    // TODO: decrement i 1_000_000 times
    for (int j = 0; j < 9; j++) {
        sem_wait(&sem);
        i--;
        printf("Decrementing: %d\n", i);
        sem_post(&sem);
        sleep(1);
    }
    return NULL;
}


int main(){
    // semaphor nr 1
    int errno = sem_init(&sem, 0, 1);
    if (errno != 0) {
        printf("Error creating semaphore:\t %s\n", strerror(errno));
        return errno;
    } 

    int error1 = pthread_create(&thread1, NULL, *incrementingThreadFunction, &i);
    int error2 = pthread_create(&thread2, NULL, *decrementingThreadFunction, &i);

    if (error1 || error2) {
        printf("Error creating threads\n Error1: %c\n Error2: %c\n", *strerror(error1), *strerror(error2));
        //return 1;
    }

    printf("%d\n", error1);
    printf("%s\n", strerror(error1));
    // TODO:
    // wait for the two threads to be done before printing the final result
    // Hint: Use `pthread_join`    

    pthread_join(thread1, NULL);
    pthread_join(thread2, NULL);
    
    sem_destroy(&sem);
    printf("The magic number is: %d\n", i);
    return 0;
}
