#include <stdlib.h>
#include <sys/socket.h>
#include <stdio.h>
#include <unistd.h>

#include "../qemu/qemu/linux-user/fitm.h"

int main() {

    char buf[100];

    char *buff = calloc(100, 1);
    sprintf(buff, "ls /proc/%d/fd", getpid());
    system(buff);
    recv(100, buf, 200, 0);
    memset(buff, 100, 0);
    sprintf(buff, "ls /proc/%d/fd", getpid());
    system(buff);

    return 0;
}