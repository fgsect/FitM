#include <unistd.h>
#include <stdio.h>
#include <sys/socket.h>

void do_syscall(void){
    // Should generate file with name <uuid> at <STATE_DIR>/fds/<uuid> and return FD
    printf("SOCKET: %d\n", socket(0, 0, 0));
    // Should always return 0
    printf("BIND: %d\n", bind(0, 0, 0));
    // Should always return 0
    printf("CONNECT: %d\n", connect(0, 0, 0));
    printf("SETSOCKOPT: %d\n", setsockopt(0, 0, 0, 0, 0));
    printf("GETSOCKOPT: %d\n", getsockopt(0, 0, 0, 0, 0));
    // Should generate file with name <uuid> at <STATE_DIR>/fds/<uuid> and return FD
    // Maybe we need to handle a connection queue or copy stuff to peer adr.
    printf("ACCEPT: %d\n", accept(0, 0, 0));
    // Write to the given FD (a local file if everything works out) and set the "sent flag"
    printf("SEND: %d\n", send(0, 0, 0, 0));
    // Read from stdin. Trigger snapshot if we've sent previously in this session
    printf("RECV: %d\n", recv(0, 0, 0, 0));
}

int main(void) {
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
