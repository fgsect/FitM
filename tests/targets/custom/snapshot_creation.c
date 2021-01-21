// Client side C/C++ program to demonstrate Socket programming
#include <stdio.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <string.h>

int main()
{
    // necessary preamble
    int sock = 0;
    char *msg = "";
    char *buffer = (char *)calloc(1, 1);
    if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0)
    {
        system("echo 'Socket creation error'");
        return -1;
    }


//    send(sock , msg, strlen(msg) , 0 );
//    printf("client sent: %s\n", msg);
    system("echo '00'");
    recv(sock, buffer, 1, 0);
    system("echo '01'");
    send(sock, msg, 1, 0);
    system("echo '02'");

//    printf("client recv #1: %s\n", buffer);

//    char *new_msg = "Need more state!\n";
//    send(sock, new_msg, strlen(new_msg), 0);
//    printf("client send #2: %s\n", new_msg);
//
//    free(buffer);
//    buffer = (char *) calloc(1, 1);
    recv(sock, buffer, 1, 0);
    system("echo '03'");
//    printf("client recv #2: %s\n", buffer);
    return 0;
}
