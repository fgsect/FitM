#!/bin/bash

# This file has to write the entire env into the envfile
env > ./envfile
echo "BASH 1"
PIPE=$(cat /tmp/log | cut -d$'\n' -f1)
# exec 188</proc/self/fd/198
# exec 189>/proc/self/fd/199
/home/hirnheiner/repos/criu/criu/criu restore -d \
    -vvv \
    -o ./restore.log \
    --images-dir $1 \
    --inherit-fd "fd[198]:$PIPE fd[199]:$PIPE" \
    && echo "OK"
echo -n "restore.sh still has the following fds: "
ls -la /proc/$$/fd
echo "BASH 2"
