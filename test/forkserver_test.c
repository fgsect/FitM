#include <stdlib.h>
#include <sys/socket.h>
#include <stdio.h>
#include <unistd.h>

#include "../qemu/qemu/linux-user/fitm.h"

int main() {

    char buf[100];
    // int pipefd[2];
    // pipe(pipefd);
    // dup2(pipefd[0], 10);
    // dup2(pipefd[1], 11);
    // close(pipefd[0]);
    // close(pipefd[1]);

    // char *buff = calloc(100, 1);
    // sprintf(buff, "ls -la /proc/%d/fd", getpid());
    // system(buff);
    recv(100, buf, 200, 0);
    // memset(buff, 100, 0);
    // sprintf(buff, "ls -la /proc/%d/fd", getpid());
    // system(buff);

    return 0;
}
