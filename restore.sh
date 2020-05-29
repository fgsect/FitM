#!/bin/bash

# This file has to write the entire env into the envfile
env > ./envfile
echo "BASH 1"
criu restore -d -vvv -o ./restore.log --images-dir $1 && echo "OK"
echo "BASH 2"