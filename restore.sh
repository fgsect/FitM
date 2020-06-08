#!/bin/bash

# This file has to write the entire env into the envfile
# Expects relative path to state folder it's supposed to restore as arg
# TODO: Clean this up, this looks/is ugly
STATE=$(echo $1 | cut -d'/' -f2)

cd states/$STATE
env > ./envfile
PIPE1=$(cat ./stdout | cut -d$'\n' -f1)
PIPE2=$(cat ./stdout | cut -d$'\n' -f2)

criu restore -d \
    -vvv \
    -o ./restore.log \
    --images-dir ./snapshot \
    --inherit-fd "fd[198]:$PIPE1" \
    --inherit-fd "fd[199]:$PIPE2" \
    && echo "OK"
