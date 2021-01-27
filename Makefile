.PHONY: all afl qemu criu fitm debug tests

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

run: tests debug
	sudo rm -rf ./active-state
	sudo rm -rf ./cmin-tmp
	sudo ./target/debug/fitm
	sudo chown -R $(USER) ./active_state
	sudo chown -R $(USER) ./saved_states


tests:
	$(MAKE) -C ./tests

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true
