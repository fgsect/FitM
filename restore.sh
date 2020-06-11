#!/bin/bash
# set -x
# This file has to write the entire env into the envfile
# Expects relative path to state folder it's supposed to restore as arg
# TODO: Clean this up, this looks/is ugly

export INPUT_FILENAME=$(realpath $2)

STATE_DIR=../../states/$1
exec 77< $STATE_DIR/out/.cur_input
exec 78< $STATE_DIR/stdout
exec 79< $STATE_DIR/stderr

env > $STATE_DIR/envfile
PIPE1=$(cat $STATE_DIR/stdout | grep "pipe:\[.*\]" | tail -n 2 | cut -d$'\n' -f1)
PIPE2=$(cat $STATE_DIR/stdout | grep "pipe:\[.*\]" | tail -n 2 | cut -d$'\n' -f2)

echo "======="
ls -la /proc/self/fd
echo "======="
cur_input=$(realpath $STATE_DIR/out/.cur_input)
stdout=$(realpath $STATE_DIR/stdout)
stderr=$(realpath $STATE_DIR/stderr)
criu restore -d \
    -vvv \
    -o ./restore.log \
    --images-dir $STATE_DIR/snapshot \
    --inherit-fd "fd[198]:$PIPE1" \
    --inherit-fd "fd[199]:$PIPE2" \
    --inherit-fd "fd[77]:${cur_input:1}" \
    --inherit-fd "fd[78]:${stdout:1}" \
    --inherit-fd "fd[79]:${stderr:1}" \
    && echo "OK"

