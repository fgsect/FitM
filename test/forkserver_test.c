#include <stdio.h>
#include <sys/socket.h>
#include <stdlib.h>

int main() {
    char *buf = (char *)calloc(100, 1);
    recv(100, buf, 100, 0);
    puts("beep");
    printf("forkserver_test received: %s\n", buf);


    if(buf[0] == 'R') {
        if (buf[1] == 'I') {
            if (buf[2] == 'P') {
                char *foo = NULL;
                printf("%s\n", foo);
            } else {
                printf("Got: RI\n");
            }
        } else {
            printf("Got: R\n");
        }
    }

    return 0;
}
