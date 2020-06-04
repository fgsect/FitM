#!/bin/bash

# This file has to write the entire env into the envfile
env > ./envfile

# TODO: Clean this up, this looks/is ugly
PIPE1=$(cat ./log | cut -d$'\n' -f1)
PIPE2=$(cat ./log | cut -d$'\n' -f2)

/home/hirnheiner/repos/criu/criu/criu restore -d \
    -vvv \
    -o ./restore.log \
    --images-dir $1 \
    --inherit-fd "fd[198]:$PIPE1" \
    --inherit-fd "fd[199]:$PIPE2" \
    && echo "OK"
