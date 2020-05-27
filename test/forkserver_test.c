#include <stdlib.h>
#include <sys/socket.h>
#include <stdio.h>

int main() {
    printf("HELLO THIS TEST STARTS HERE!!!\n");

    char buf[100];
    recv(100, buf, 200, 0);

    printf("NEW TEST WHO DIS?\n");

    return 0;
}