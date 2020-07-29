#include <stdlib.h>
#include <sys/socket.h>
#include <stdio.h>
#include <unistd.h>

#include "../qemu/qemu/linux-user/fitm.h"


void child(int *pipefd) {
    char *buff = calloc(100, 1);
    char buf[100];

    int pid = getpid();
    sprintf(buff, "ls /proc/%d/fd", pid);
    printf("* child pid: %d\n", pid);
    char* msg = "foo";
    write(pipefd[1], msg, strlen(msg));

    recv(100, buf, 200, 0);
    msg = "bar";
    write(pipefd[1], msg, strlen(msg));
    memset(buff, 100, 0);


    sprintf(buff, "ls /proc/%d/fd", getpid());
    // system(buff);
}


int main() {
    char buf[100];

    char *buff = calloc(100, 1);

    int pipefd[2];
    pipe(pipefd);
    int childPid = fork();
    if (childPid == -1) {
    } else if ( childPid == 0) {
        char *msg = calloc(100, 1);
        read(pipefd[0], msg, 100);
        printf("+ msg from pipe: %s\n", msg);
        // close(pipefd[1]);
        int pid = getpid();
        sprintf(buff, "ls /proc/%d/fd", pid);
        system(buff);
        printf("+ parent pid: %d\n", pid);

        sleep(2);
        char *criu_call = calloc(150, 1);
        strcpy(criu_call, "sudo /home/hirnheiner/repos/criu/criu/criu restore -d -vvv -o restore.log --images-dir /tmp/criu_snapshot --inherit-fd ");
        char* path = calloc(100, 1);
        sprintf(path, "/proc/self/fd/%i", pipefd[1]);

        char* ls = calloc(100, 1);
        sprintf(ls, "ls /proc/self/fd");
        system(ls);

        char* pipename = calloc(100, 1);
        readlink(path, pipename, 100);
        printf("%s\n", path);
        printf("%s\n", pipename);

        char* inheritfd_arg = calloc(100, 1);
        sprintf(inheritfd_arg, "fd[%i]:%s", pipefd[1], pipename);
        strncat(criu_call, inheritfd_arg, strlen(inheritfd_arg));
        system(criu_call);
        while(1){
            read(pipefd[0], msg, 100);
            printf("+ msg from pipe: %s\n", msg);
        };
    } else {
        close(pipefd[0]);
        child(pipefd);
    }

    return 0;
}
