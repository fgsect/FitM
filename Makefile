.PHONY: all afl qemu criu fitm debug tests subinit

CRIUPATH?=./criu

all: criu symlink qemu afl

subinit:
	git submodule init
	git submodule update

afl: subinit
	make -C ./AFLplusplus

qemu: criu subinit
	cd ./qemu/qemu/ && ./build-for-afl.sh

criu: subinit
	make -C ./criu

fitm:
	cargo build --release

debug:
	cargo build

run: fitm #tests debug
	sudo rm -rf ./active-state
	sudo rm -rf ./cmin-tmp
	sudo ./target/release/fitm
	sudo chown -R $(USER) ./active_state
	sudo chown -R $(USER) ./saved_states


tests:
	$(MAKE) -C ./tests

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true
