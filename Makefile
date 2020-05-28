build:
	cd ./qemu/qemu/ && \
	./build-for-afl.sh && \
	cd ../..

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true