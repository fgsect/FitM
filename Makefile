CRIUPATH?=./criu

build: build_afl build_qemu build_criu

build_afl:
	make -C ./AFLplusplus

build_qemu:
	cd ./qemu/qemu/ && ./build-for-afl.sh

build_criu:
	make -C ./criu

fitm:
	cargo build --release

debug:
	cargo build

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true