.PHONY: all afl qemu criu fitm debug

CRIUPATH?=./criu

all: criu symlink qemu afl

afl:
	make -C ./AFLplusplus

qemu:
	cd ./qemu/qemu/ && ./build-for-afl.sh

criu:
	make -C ./criu

fitm:
	cargo build --release

debug:
	cargo build

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true
