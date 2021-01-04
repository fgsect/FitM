CRIUPATH?=./criu

all: build_afl build_qemu build_criu

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