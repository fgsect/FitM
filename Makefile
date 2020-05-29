build: build_afl build_qemu

build_afl:
	cd AFLplusplus && make

build_qemu:
	cd ./qemu/qemu/ && ./build-for-afl.sh

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true