#include <stdlib.h>
#include <sys/socket.h>
#include <string.h>
#include <stdio.h>

int main()
{
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    char buf[500];
    recv(sock, buf, 500, 0);

//    printf("randomsg: %s\n", buf);
    if(!strcmp(buf, "abcde")){
        exit(0);
    }

    char *new_msg = "ACK! Got correct init signal\n";
    send(sock , new_msg , strlen(new_msg) , 0);

    recv(sock, buf, 100, 0);
    return sock;
}
