#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <string.h>

int main() {

    // open socket
    int server_fd, new_socket;
    struct sockaddr_in address;
    int addrlen = sizeof(address);
    int opt = 1;
    if ((server_fd = socket(AF_INET, SOCK_STREAM, 0)) == 0)
    {
        perror("socket failed");
        exit(EXIT_FAILURE);
    }

    // Forcefully attaching socket to the port 8080
    if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT,
               &opt, sizeof(opt)))
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

    char *buf = (char *)calloc(100, 1);
    // client sends first...
//    send(new_socket, "RI\0", 3, 0);

    // recv 1
    recv(new_socket, buf, 100, 0);
    printf("server recv #1: %s\n", buf);

    if(buf[0] == 'R') {
        free(buf);
        buf = (char *)calloc(100, 1);

        // send 1
        char *new_msg = "ACK! Got correct init signal\n";
        send(new_socket , new_msg , strlen(new_msg) , 0 );
        printf("server send #1: %s\n", new_msg);

        free(buf);
        buf = (char *)calloc(100, 1);
        // recv 2
        recv(new_socket, buf, 100, 0);
        printf("server recv #2: %s\n", buf);
        if (!strcmp(buf, "Need more state!\n")) {

            // send 2
            new_msg = "make client go b0000000000m.\n";
            send(new_socket , new_msg , strlen(new_msg) , 0 );
            printf("server send #2: %s\n", new_msg);

            free(buf);
        } else {
            printf("server didn't get more state\n");
        }
    } else {
        printf("server got incorrect init signal...aborting\n");
    }
    return 0;
}
