#include <unistd.h>
#include <stdio.h>
#include <sys/socket.h>
#include <stdlib.h>
#include <netinet/in.h>
#include <string.h>

void do_syscall(void){
    // Should generate file with name <uuid> at <STATE_DIR>/fds/<uuid> and return FD
    printf("SOCKET: %d\n", socket(0, 0, 0));
    // Should always return 0
    printf("BIND: %d\n", bind(0, 0, 0));
    // Should always return 0
    printf("CONNECT: %d\n", connect(0, 0, 0));
    // Should always return 0
    printf("SETSOCKOPT: %d\n", setsockopt(0, 0, 0, 0, 0));
    // Should always return 0
    printf("GETSOCKOPT: %d\n", getsockopt(0, 0, 0, 0, 0));
    // Should generate file with name <uuid> at <STATE_DIR>/fds/<uuid> and return FD
    // Maybe we need to handle a connection queue or copy stuff to peer adr.
    printf("ACCEPT: %d\n", accept(0, 0, 0));
    // Write to the given FD (a local file if everything works out) and set the "sent flag"
    printf("SEND: %d\n", send(0, 0, 0, 0));
    // Read from stdin. Trigger snapshot if we've sent previously in this session
    printf("RECV: %d\n", recv(0, 0, 0, 0));
    // Should always return 0
    printf("LISTEN: %d\n", listen(0, 0));
}

void run_server(void){
    int server_fd, new_socket, valread;
    struct sockaddr_in address;
    int opt = 1;
    int addrlen = sizeof(address);
    char buffer[1024] = {0};
    char *hello = "Hello from server";

    // Creating socket file descriptor
    if ((server_fd = socket(AF_INET, SOCK_STREAM, 0)) == 0)
    {
        perror("socket failed");
        exit(EXIT_FAILURE);
    }

    // Forcefully attaching socket to the port 8080
    if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)))
    {
        perror("setsockopt");
        exit(EXIT_FAILURE);
    }
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = INADDR_ANY;
    address.sin_port = htons( 8080 );

    // Forcefully attaching socket to the port 8080
    if (bind(server_fd, (struct sockaddr *)&address,
                                 sizeof(address))<0)
    {
        perror("bind failed");
        exit(EXIT_FAILURE);
    }
    if (listen(server_fd, 3) < 0)
    {
        perror("listen");
        exit(EXIT_FAILURE);
    }
    if ((new_socket = accept(server_fd, (struct sockaddr *)&address,
                       (socklen_t*)&addrlen))<0)
    {
        perror("accept");
        exit(EXIT_FAILURE);
    }
    valread = read( new_socket , buffer, 1024);
    printf("%s\n",buffer );
    send(new_socket , hello , strlen(hello) , 0 );
    printf("Hello message sent\n");
}

int main(void) {
    int state = 0;
    while(state < 2){
        printf("%i\n", state);
        state++;
        sleep(1);
    }
    run_server();
    while(state < 6){
        printf("%i\n", state);
        state++;
        sleep(1);
    }
}
