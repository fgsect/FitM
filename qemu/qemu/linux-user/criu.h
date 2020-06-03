//
// Created by hirnheiner on 11.05.20.
// Checkout the Makefile

#include "rpc.pb-c.h"
#include <stdlib.h>
#include <stdbool.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <fcntl.h>
#include <string.h>
#include <stdio.h>
#include <unistd.h>
#include <uuid/uuid.h>


#define MAX_MSG_SIZE 1024

char* get_new_uuid(void);
int do_criu(void);
char* concat3(char *first, char *second, char *third);


static int send_req(int socket_fd, CriuReq *req)
{
    unsigned char buf[MAX_MSG_SIZE];
    int len;

    len = criu_req__get_packed_size(req);

    if (criu_req__pack(req, buf) != len) {
        perror("Failed packing request");
        return -1;
    }

    if (write(socket_fd, buf, len)  == -1) {
        perror("Can't send request");
        return -1;
    }

    return 0;
}

static CriuResp *recv_resp(int socket_fd)
{
	unsigned char buf[MAX_MSG_SIZE];
	int len;
	CriuResp *msg = 0;

	len = read(socket_fd, buf, MAX_MSG_SIZE);
	if (len == -1) {
		perror("Can't read response");
		return NULL;
	}

	msg = criu_resp__unpack(NULL, len, buf);
	if (!msg) {
		perror("Failed unpacking response");
		return NULL;
	}

	return msg;
}

int do_criu(void){
    // int pid = fork();
    int ret = 0;
    // if (pid) {
    // wait for child
//     int status;
//     waitpid(pid, &status, 0);
//     if (WIFEXITED(status))
//         status = WEXITSTATUS(status);
//     if (status)
//         exit(status);
// } else {
    CriuReq req		= CRIU_REQ__INIT;
    CriuResp *resp		= NULL;
    int fd, dir_fd;
    struct sockaddr_un addr;
    socklen_t addr_len;

    dir_fd = open("/tmp/criu_snapshot", O_DIRECTORY);
    if (dir_fd == -1) {
        perror("Can't open /tmp/criu_snapshot dir");
        return -1;
    }

    req.type			= CRIU_REQ_TYPE__DUMP;
    req.opts			= malloc(sizeof(CriuOpts));
    if (!req.opts) {
        perror("Can't allocate memory for dump request");
        return -1;
    }

    criu_opts__init(req.opts);
    req.opts->images_dir_fd		= dir_fd;
    req.opts->log_level		= 4;
    req.opts->leave_running = true;

    fd = socket(AF_LOCAL, SOCK_SEQPACKET, 0);
    if (fd == -1) {
        perror("Can't create socket");
        return -1;
    }

    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_LOCAL;

    strcpy(addr.sun_path, "/tmp/criu_service.socket");

    addr_len = strlen(addr.sun_path) + sizeof(addr.sun_family);

    ret = connect(fd, (struct sockaddr *) &addr, addr_len);
    if (ret == -1) {
        perror("Can't connect to socket");
        goto exit;
    }

    /*
    * Send request
    */
    ret = send_req(fd, &req);
    if (ret == -1) {
        perror("Can't send request");
        goto exit;
    }

    /*
    * Recv response
    */
    resp = recv_resp(fd);
    if (!resp) {
        perror("Can't recv response");
        ret = -1;
        goto exit;
    }

    if (resp->type != CRIU_REQ_TYPE__DUMP) {
        perror("Unexpected response type");
        ret = -1;
        goto exit;
    }

    /*
    * Check response.
    */
    if (resp->success)
        puts("Success");
    else {
        puts("Fail");
        ret = -1;
        goto exit;
    }

    if (resp->dump->has_restored && resp->dump->restored)
        puts("Restored");


    // Closing the socket FD before the process is dumped breaks CRIU
exit:
    close(fd);
    close(dir_fd);
    if (resp)
        criu_resp__free_unpacked(resp, NULL);

// }
return ret;
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
    char *ret = (char *)calloc(strlen(first)+strlen(second)+strlen(third)+1, 1);
    strncpy(ret, first, strlen(first)+1);
    strncat(ret, second, strlen(second)+1);
    strncat(ret, third, strlen(third)+1);
    return ret;
}
