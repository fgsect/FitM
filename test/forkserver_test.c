#include <stdlib.h>
#include <sys/socket.h>
#include <stdio.h>

int main() {
    printf("HELLO THIS TEST STARTS HERE!!!\n");

    char buf[100];
    puts("pre recv:");
    char* tmp = getenv("LETS_DO_THE_TIMEWARP_AGAIN");
    if(tmp){
        printf("env val: %s\n", tmp);
    } else{
        puts("no LDTTA var..");
    }

    recv(100, buf, 200, 0);

    puts("post recv:");
    tmp = getenv("LETS_DO_THE_TIMEWARP_AGAIN");
    if(tmp){
        printf("env val: %s\n", tmp);
    } else{
        puts("no LDTTA var..");
    }
    printf("NEW TEST WHO DIS?\n");

    return 0;
}