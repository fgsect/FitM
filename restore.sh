#!/bin/bash

# This file has to write the entire env into the envfile
env > ./envfile
echo "1"
/home/hirnheiner/repos/criu/criu/criu restore -d -vvv -o restore.log --images-dir $1 && echo "OK"
echo "2"