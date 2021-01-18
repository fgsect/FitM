//
// Created by hirnheiner on 11.05.20.
// Checkout the Makefile
#include "criu.h"
// #include "rpc.pb-c.h"
#include <stdlib.h>
#include <stdbool.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <fcntl.h>
#include <string.h>
#include <stdio.h>
#include <unistd.h>
#include <uuid/uuid.h>

#include "fitm.h"



#define SNAP_SUCCESS_EXIT 42
#define MAX_MSG_SIZE 1024

char* get_new_uuid(void);
int do_criu(void);
char* concat3(char *first, char *second, char *third);
void save_exitcode(int exitcode);


void save_exitcode(int exitcode){
    FILE *fd = fopen("target-exitcode", "w");
    if(!fd) {
        perror("Could not open target-exitcode in fitm-criu.h\n");
    }

    // https://stackoverflow.com/a/32819876
    char buffer[snprintf(NULL, 0, "%d", exitcode)+1];
    sprintf(buffer, "%d", exitcode);
    // don't write null byte into file
    fwrite(buffer, 1, sizeof(buffer)-1, fd);
    if(ferror(fd)) {
        perror("Error occured while writing to target-exitcode in fitm-criu.h\n");
    }
    fclose(fd);
}

int do_criu(void){
    int dir_fd, exitcode;
    struct criu_opts *criu_request_options = NULL;

    char *uuid = get_new_uuid();
    char path[44] = "/tmp/";
    strncat(path, uuid, 37);
    close(open(path, O_RDWR | O_CREAT, 0644));

    char *snapshot_dir = getenv_from_file("CRIU_SNAPSHOT_OUT_DIR");

    dir_fd = open(snapshot_dir, O_DIRECTORY);
    if (dir_fd == -1) {
        perror("Can't open snapshot dir\n");
        exitcode = -1;
        goto exit;
    }

    if (criu_local_init_opts(&criu_request_options)) {
        perror("Can't allocate memory for dump request\n");
        exitcode = -1;
        goto exit;
    }

    if (criu_local_set_service_address(criu_request_options, "/tmp/criu_service.socket")) {
        perror("Couldn't set service address\n");
        exitcode = -1;
        goto exit;
    }

    criu_local_set_images_dir_fd(criu_request_options, dir_fd);
    criu_local_set_log_level(criu_request_options, 4);
    criu_local_set_leave_running(criu_request_options, true);
    
    int criu_result = criu_local_dump(criu_request_options);

    if (criu_result < 0) {
        printf("An error in criu has occured %d\n", criu_result);
        exitcode = -1;
        goto exit;
    }
    
    if (criu_result == 0) {
        // SIGNAL INIT
        // Internet says we should rely on files or others in this case: https://stackoverflow.com/a/7697135
        // write exit code to file to read out in fitm
        save_exitcode(SNAP_SUCCESS_EXIT);

        /* We exit with 42 upon a successful snapshot-exit
        The returncode is checked in snapshot_run to determine 
        whether a new checkpoint was reached */
        exit(SNAP_SUCCESS_EXIT);
    }

    if (criu_result == 1) {
        printf("RESTORED\n");
        close(dir_fd);
        criu_local_free_opts(criu_request_options);
        exitcode = 0;
        save_exitcode(exitcode);
        return exitcode;
    }

    printf("Unexpected criu-result %d\n", criu_result);

exit:
    save_exitcode(exitcode);
    _exit(exitcode);
}

char* get_new_uuid(void){
    // Taken from: https://stackoverflow.com/questions/51053568/generating-a-random-uuid-in-c
    uuid_t binuuid;
    uuid_generate_random(binuuid);

    char *uuid = malloc(37);
    uuid_unparse_lower(binuuid, uuid);
    return uuid;
}

char* concat3(char *first, char *second, char *third){
    char *ret = (char *)calloc(strlen(first)+strlen(second)+strlen(third)+4, 1);
    strncpy(ret, first, strlen(first)+1);
    strncat(ret, second, strlen(second)+1);
    strncat(ret, third, strlen(third)+1);
    return ret;
}
