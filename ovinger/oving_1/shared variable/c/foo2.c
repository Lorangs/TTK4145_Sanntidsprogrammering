// Compile with `gcc foo.c -Wall -std=gnu99 -lpthread`, or use the makefile
// The executable will be named `foo` if you use the makefile, or `a.out` if you use gcc directly

// test med mutexes. 

#include <pthread.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>

int i = 0;
pthread_t thread1;
pthread_t thread2;
pthread_mutex_t mutex;

// Note the return type: void*
void* incrementingThreadFunction(){
    // TODO: increment i 1_000_000 timesmak
    for (int j = 0; j < 10; j++) {
        pthread_mutex_lock(&mutex);
        i++;
        printf("Incrementing: %d\n", i);
        pthread_mutex_unlock(&mutex);
        sleep(1);
    }
    return NULL;
}

void* decrementingThreadFunction(){
    // TODO: decrement i 1_000_000 times
    for (int j = 0; j < 9; j++) {
        pthread_mutex_lock(&mutex);
        i--;
        printf("Decrementing: %d\n", i);
        pthread_mutex_unlock(&mutex);
        sleep(1);
    }
    return NULL;
}


int main(){
    // TODO: 
    // start the two functions as their own threads using `pthread_create`
    // Hint: search the web! Maybe try "pthread_create example"?
    pthread_mutex_init(&mutex, NULL);

    int error1 = pthread_create(&thread1, NULL, *incrementingThreadFunction, &i);
    int error2 = pthread_create(&thread2, NULL, *decrementingThreadFunction, &i);

    error1  = 2;
    if (error1 || error2) {
        printf("Error creating threads\n Error1: %s\n Error2: %s\n", strerror(error1), strerror(error2));
        //return 1;
    }

    pthread_join(thread1, NULL);
    pthread_join(thread2, NULL);
    
    pthread_mutex_destroy(&mutex);
    printf("The magic number is: %d\n", i);
    return 0;
}
