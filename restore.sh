#!/bin/bash
# set -x
# This file has to write the entire env into the envfile
# Expects relative path to state folder it's supposed to restore as arg
# TODO: Clean this up, this looks/is ugly

export INPUT_FILENAME=$(realpath $2)

STATE_DIR=../../states/$1

env > $STATE_DIR/envfile
PIPE1=$(cat $STATE_DIR/pipes | grep "pipe:\[.*\]" | tail -n 2 | cut -d$'\n' -f1)
PIPE2=$(cat $STATE_DIR/pipes | grep "pipe:\[.*\]" | tail -n 2 | cut -d$'\n' -f2)

echo "======="
ls -la /proc/self/fd
echo "======="

echo -n "" > ./out/.cur_input

criu restore -d \
    -vvv \
    -o ./restore.log \
    --images-dir $STATE_DIR/snapshot \
    --inherit-fd "fd[198]:$PIPE1" \
    --inherit-fd "fd[199]:$PIPE2" \
    && echo "OK"
