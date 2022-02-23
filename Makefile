.PHONY: all afl qemu criu fitm debug tests subinit

CRIUPATH?=./criu

all: criu symlink qemu afl tests fitm

subinit:
	git submodule init || true
	git submodule update || true

afl: subinit
	make -C ./AFLplusplus

fitm-qemu-trace:
	$(MAKE) criu
	cd ./fitm-qemu && ./build_qemu_support.sh

qemu: fitm-qemu-trace criu subinit
	# rebuild each time, lightly
	cd ./fitm-qemu && ./build_incremental.sh

criu: subinit
	make -C ./criu

fitm:
	cargo build --release

debug:
	cargo build

reset:
	sudo rm fitm-state.json || true
	sudo rm -rf ./active-state
	sudo rm -rf ./saved-states
	sudo rm -rf ./cmin-tmp

run: fitm #tests debug
	sudo rm -rf ./active-state
	sudo rm -rf ./cmin-tmp
	sudo ./target/release/fitm ./Orthanc.fitm-args.json
	# sudo ./target/release/fitm ./kamailio.fitm-args.json
	# sudo ./target/release/fitm ./ts3.fitm-args.json
	# sudo ./target/release/fitm ./fitm-args.rtsp.json
	sudo chown -R pleb ./active_state
	sudo chown -R pleb ./saved_states

clear_state:
	rm -rf ./active-state/ ./generation_inputs/ ./saved-states/ ./cmin-tmp/ ./fitm-state.json

tests:
	$(MAKE) -C ./tests

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink: criu
	ln -s $(CRIUPATH)/images/rpc.proto || true
