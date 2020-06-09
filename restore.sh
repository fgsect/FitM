#!/bin/bash
# set -x
# This file has to write the entire env into the envfile
# Expects relative path to state folder it's supposed to restore as arg
# TODO: Clean this up, this looks/is ugly
STATE=$(echo $1 | cut -d'/' -f2)

export INPUT_FILENAME=$(realpath $2)
env > states/$STATE/envfile
PIPE1=$(cat states/$STATE/stdout | cut -d$'\n' -f1)
PIPE2=$(cat states/$STATE/stdout | cut -d$'\n' -f2)

ls -la /proc/self/fd

/home/hirnheiner/repos/criu/criu/criu restore -d \
    -vvv \
    -o ./restore.log \
    --images-dir states/$STATE/snapshot \
    --inherit-fd "fd[198]:$PIPE1" \
    --inherit-fd "fd[199]:$PIPE2" \
    && echo "OK"

