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

#define MAX_MSG_SIZE 1024

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

int do_criu(){
    CriuReq req		= CRIU_REQ__INIT;
    CriuResp *resp		= NULL;
    int fd, dir_fd;
    int ret = 0;
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


exit:
    // Closing the socket FD before the process is dumped breaks CRIU
//    close(fd);
//    close(dir_fd);
    if (resp)
        criu_resp__free_unpacked(resp, NULL);
    return ret;
}


int main(int argc, char *argv[]) {
    int state = 0;
    while(state < 2){
        printf("%i\n", state++);
        sleep(1);
    }
    puts("requesting process dump");

    // request selfdump
    do_criu();

    puts("continue...");
    while(state < 10){
        printf("%i\n", state++);
        sleep(1);
    }
}
