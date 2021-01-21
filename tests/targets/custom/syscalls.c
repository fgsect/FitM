#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <sys/types.h>

// getsid() - returns the Session ID
// sauce: http://dcjtech.info/topic/using-linux-syscalls-in-c-programming/
int main(int argc, char *argv[]) {
    int pid, sid;
    if (argc > 2) {
        printf("Expected one or no parameters, but was given %d\n", argc - 1);
        return 1;
    } else if (argc == 1) {
        pid = 0;
    } else {
        pid = atoi(argv[1]);
    }
    pid_t getsid(pid_t pid);
    sid = getsid(pid);
    printf("%d\n", sid);
    return 0;
}