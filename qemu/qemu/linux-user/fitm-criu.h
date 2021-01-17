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



#define MAX_MSG_SIZE 1024

char* get_new_uuid(void);
int do_criu(void);
char* concat3(char *first, char *second, char *third);


// static int send_req(int socket_fd, CriuReq *req)
// {
//     unsigned char buf[MAX_MSG_SIZE];
//     int len;

//     len = criu_req__get_packed_size(req);

//     if (criu_req__pack(req, buf) != len) {
//         perror("Failed packing request");
//         return -1;
//     }

//     if (write(socket_fd, buf, len)  == -1) {
//         perror("Can't send request");
//         return -1;
//     }

//     return 0;
// }

// static CriuResp *recv_resp(int socket_fd)
// {
// 	unsigned char buf[MAX_MSG_SIZE];
// 	int len;
// 	CriuResp *msg = 0;

// 	len = read(socket_fd, buf, MAX_MSG_SIZE);
// 	if (len == -1) {
// 		perror("Can't read response");
// 		return NULL;
// 	}

// 	msg = criu_resp__unpack(NULL, len, buf);
// 	if (!msg) {
// 		perror("Failed unpacking response");
// 		return NULL;
// 	}

// 	return msg;
// }

int do_criu(void){
    int ret = 1;
    int fd, dir_fd;
    struct criu_opts *criu_request_options = NULL;
    struct sockaddr_un addr;
    socklen_t addr_len;

    char *uuid = get_new_uuid();
    char path[44] = "/tmp/";
    strncat(path, uuid, 37);
    close(open(path, O_RDWR | O_CREAT, 0644));

    char *snapshot_dir = getenv_from_file("CRIU_SNAPSHOT_OUT_DIR");

    dir_fd = open(snapshot_dir, O_DIRECTORY);
    if (dir_fd == -1) {
        perror("Can't open snapshot dir");
        goto exit;
    }

    if (criu_local_init_opts(&criu_request_options)) {
        perror("Can't allocate memory for dump request");
        goto exit;
    }

    if (criu_local_set_service_address(criu_request_options, "/tmp/criu_service.socket")) {
        perror("Couldn't set service address");
        goto exit;
    }

    criu_local_set_images_dir_fd(criu_request_options, dir_fd);
    criu_local_set_log_level(criu_request_options, 4);
    criu_local_set_leave_running(criu_request_options, true);
    
    int criu_result = criu_local_dump(criu_request_options);
    printf("Criu-result: %d", criu_result);
    
    if (criu_result < 0) {
        printf("An error in criu has occured %d\n", criu_result);
        goto exit;
    }
    
    if (criu_result == 0) {
        printf("Snapshot successful\n");
        printf("EXITING\n");

        // SIGNAL INIT

        /* We exit with 42 upon a successful snapshot-exit
        The returncode is checked in snapshot_run to determine 
        whether a new checkpoint was reached */
        exit(42);
    }

    if (criu_result == 1) {
        printf("RESTORED\n");
        close(dir_fd);
        criu_local_free_opts(criu_request_options);
        return 0;
    }

    printf("Unexpected criu-result %d", criu_result);

exit:
    _exit(-1);
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
