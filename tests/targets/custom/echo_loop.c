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
    char *buffer = (char *)calloc(1024, 1);
    if (!buffer) {
        perror("OOM");
        return -1;
    }
    if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0)
    {
        perror("Socket creation error");
        return -1;
    }

    struct sockaddr_in sock_addr = {
        .sin_addr.s_addr = htonl(INADDR_ANY),
        .sin_port = 1337,
        .sin_family = AF_INET,
    };

    if (connect(sock, (struct sockaddr *) &sock_addr, sizeof(struct sockaddr_in)) < 0) {
        perror("Connect failed");
        return 1;
    }
    if (send(sock, "FITM", 4, 0) <= 0) {
        perror("Could not send any data");
        return 1;
    }

    while (1) {
        int bytes_recieved = recv(sock, buffer, 1024, 0);
        if (bytes_recieved <= 0) {
            break;
        }

        int bytes_sent = send(sock, buffer, bytes_recieved, 0);
        if (bytes_sent <= 0) {
            break;
        }
    }

    return 0;
}
