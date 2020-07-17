
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
    // necessary preamble
    int sock = 0;
    struct sockaddr_in serv_addr;
    char *msg = "";
    char *buffer = (char *)calloc(1, 1);
    if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0)
    {
        printf("\n Socket creation error \n");
        return -1;
    }


//    send(sock , msg, strlen(msg) , 0 );
//    printf("client sent: %s\n", msg);

    puts("00");
    recv(sock, buffer, 1, 0);
    puts("01");
    printf("client recv #1: %s\n", buffer);

//    char *new_msg = "Need more state!\n";
//    send(sock, new_msg, strlen(new_msg), 0);
//    printf("client send #2: %s\n", new_msg);
//
//    free(buffer);
//    buffer = (char *) calloc(1, 1);
//
//    recv(sock, buffer, 0, 0);
//    printf("client recv #2: %s\n", buffer);
    return 0;
}
