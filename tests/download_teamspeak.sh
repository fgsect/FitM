#!/bin/sh

TEAMSPEAK_TAR='teamspeak3-server_linux_amd64-3.13.3.tar.bz2'
TEAMSPEAK_URL='https://files.teamspeak-services.com/releases/server/3.13.3/teamspeak3-server_linux_amd64-3.13.3.tar.bz2'

cd targets
wget "$TEAMSPEAK_URL"
tar -xvf "$TEAMSPEAK_TAR"
rm "$TEAMSPEAK_TAR"
cd ..