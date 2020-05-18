#include <unistd.h>
#include <stdio.h>
#include <sys/socket.h>

void do_syscall(){
    printf("SOCKET: %d\n", socket(0, 0, 0));
    printf("BIND: %d\n", bind(0, 0, 0));
    printf("CONNECT: %d\n", connect(0, 0, 0));
    printf("SETSOCKOPT: %d\n", setsockopt(0, 0, 0, 0, 0));
    printf("GETSOCKOPT: %d\n", getsockopt(0, 0, 0, 0, 0));
    printf("ACCEPT: %d\n", accept(0, 0, 0));
    printf("SEND: %d\n", send(0, 0, 0, 0));
    printf("RECV: %d\n", recv(0, 0, 0, 0));
}

int main() {
    int state = 0;
    while(state < 2){
        printf("%i\n", state);
        state++;
        sleep(1);
    }
    do_syscall();
    while(state < 6){
        printf("%i\n", state);
        state++;
        sleep(1);
    }
}
