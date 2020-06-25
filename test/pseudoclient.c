
// Client side C/C++ program to demonstrate Socket programming
#include <stdio.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <string.h>
#define PORT 8080

int main()
{
    int sock = 0;
    struct sockaddr_in serv_addr;
    char *msg = "R";
    char *buffer = (char *)calloc(100, 1);
    if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0)
    {
        printf("\n Socket creation error \n");
        return -1;
    }

    serv_addr.sin_family = AF_INET;
    serv_addr.sin_port = htons(PORT);

    // Convert IPv4 and IPv6 addresses from text to binary form
    if(inet_pton(AF_INET, "127.0.0.1", &serv_addr.sin_addr)<=0)
    {
        printf("\nInvalid address / Address not supported \n");
        return -1;
    }

    if (connect(sock, (struct sockaddr *)&serv_addr, sizeof(serv_addr)) < 0)
    {
        printf("\nConnection Failed \n");
        return -1;
    }
    // TODO: This is a quick fix for our init_run being developed only with server binaries in mind
    // I am not sure atm if `recv` is the right point to snapshot the client.
    send(sock , msg , strlen(msg) , 0 );
    printf("Client sent: %s\n", msg);
    recv(sock, buffer, 100, 0);
    if(strcmp(msg, "ACK! Got correct init signal\n")) {
        printf("recv #1: %s\n", buffer);
        char *new_msg = "Need more state!\n";
        send(sock, new_msg, strlen(new_msg), 0);
        printf("send #2: %s\n", new_msg);

        free(buffer);
        buffer = (char *) calloc(100, 1);

        recv(sock, buffer, 100, 0);
        if (strcmp(buffer, "make client go b00m.\n\n")) {
            printf("dingdingding, client goes bum");
            buffer[9999];
        }
    }
    return 0;
}
