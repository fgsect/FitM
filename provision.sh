#!/bin/sh
chown -Rv _apt:root /var/cache/apt/archives/partial/
chmod -Rv 700 /var/cache/apt/archives/partial/
apt-get -y update && apt-get -y upgrade
apt-get -y install ntp # get rid of clock-skew in the vm
apt-get -y install build-essential binutils pkg-config python-ipaddress make protobuf-compiler protobuf-c-compiler protobuf-compiler libprotobuf-c-dev libprotobuf-dev libnet-dev python3-protobuf python3-yaml protobuf-c-compiler libbsd-dev libprotobuf-dev libprotobuf-c-dev protobuf-c-compiler protobuf-compiler python-protobuf libnl-3-dev libcap-dev ninja-build libglib2.0-dev cmake libcapstone-dev
sudo -u vagrant -- sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"

# target deps
apt-get -y install libgnutls28-dev bison flex libssl-dev autoconf libtool libsdl2-dev || true