#include <stdlib.h>
#include <stdio.h>
#include "../qemu/qemu/linux-user/fitm.h"

int main() {
    char* foo = getenv_from_file("TESTENV");
    if(foo){
        printf("%s\n", foo);
    } else {
        puts("var not found");
    }
    return 0;
}