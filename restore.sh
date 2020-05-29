#!/bin/bash

# This file has to write the entire env into the envfile
env > ./envfile
echo "BASH 1"
criu restore -d \
    -vvv \
    -o ./restore.log \
    --images-dir $1 \
    --inherit-fd fd[198]:$(readlink /proc/$$/fd/198) \
    --inherit-fd fd[199]:$(readlink /proc/$$/fd/199) \
    && echo "OK"
echo -n "Bash still has the following fds: "
ls /proc/$$/fd
echo "BASH 2"