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
    char *buffer = (char *)calloc(100, 1);
    if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0)
    {
        system("echo 'Socket creation error'");
        return -1;
    }

//    printf("00: pre recv\n");
    recv(sock, buffer, 100, 0);
//    printf("01: received: %s\n", buffer);
//    printf("02: post recv/pre send\n");
//    printf("%i\n", sock);
    system("whoami > /tmp/fitm-who");
    int bytes = send(sock, buffer, 100, 0);
    printf("sent bytes: %d\n", bytes);
    perror("error: ");
//    write(3, (char *) msg, len);
//    printf("03: post send\n");

    return 0;
}
