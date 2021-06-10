all: lightftp live555 pseudotest_simple kamailio pjproject teamspeak tsclientlib

./targets/pjproject/pjsip-apps/bin/pjsua-x86_64-unknown-linux-gnu:
	cd ./targets/pjproject && ./configure && $(MAKE) dep && $(MAKE) clean && $(MAKE) && cd ../../

pjproject: ./targets/pjproject/pjsip-apps/bin/pjsua-x86_64-unknown-linux-gnu

./targets/kamailio/src/kamailio:
	$(MAKE) -C ./targets/kamailio cfg
	$(MAKE) -C ./targets/kamailio all

kamailio: ./targets/kamailio/src/kamailio

./targets/tsclientlib/target/release/examples/simple-sync:
	git submodule init && git submodule update targets/tsclientlib
	cd targets/tsclientlib && git submodule init && git submodule update && cd ../../
	cd targets/tsclientlib && cargo build --release --examples

tsclientlib: ./targets/tsclientlib/target/release/examples/simple-sync

./targets/teamspeak3-server_linux_amd64/ts3server:
	./download_teamspeak.sh

teamspeak: ./targets/teamspeak3-server_linux_amd64/ts3server

lightftp:
	$(MAKE) -C ./targets/LightFTP/Source/Release clean all

live555:
	cd ./targets/live555 && ./genMakefiles linux-64bit
	$(MAKE) -C ./targets/live555
	$(MAKE) -C ./targets/live555/testProgs

live555-client:
	./targets/live555/testProgs/testRTSPClient rtsp://192.168.178.27:8554/wavAudioTest

live555-server:
	cd ./targets/live555/testProgs && ./testOnDemandRTSPServer 8854 && cd ../../

selfdump: symlink
	mkdir -p /tmp/criu_snapshot
	protoc-c --proto_path=. --c_out=. rpc.proto
	cc -g -Wall -I.   -c -o self-dump.o self-dump.c
	cc -g -Wall -I.   -c -o rpc.pb-c.o rpc.pb-c.c
	cc self-dump.o rpc.pb-c.o  -lprotobuf-c -o self-dump

syscall:
	gcc -o syscall syscall_test.c

clean:
	rm -rf self-dump self-dump.o rpc.pb-c.o rpc.pb-c.c rpc.pb-c.h \
	forkserver_test rpc.proto syscall getenv_from_file_test pipe_test \
	targets/custom/pseudoclient targets/custom/pseudoserver targets/custom/snapshot_creation

test_getenv: clean
	gcc -o getenv_from_file_test getenv_from_file_test.c
	./getenv_from_file_test

test_pipe: clean
	gcc -o pipe_test pipe_test.c
	setsid stdbuf -oL ../AFLplusplus/afl-qemu-trace ./pipe_test < /dev/null &> pipe_test.out

snapshot_creation:
	gcc -o targets/custom/snapshot_creation targets/custom/snapshot_creation.c

echo_server:
	gcc -o targets/custom/echo_server targets/custom/echo_server.c

pseudotest_simple: pseudoserver_simple pseudoclient_simple

pseudoserver_simple: clean
	gcc -o targets/custom/pseudoserver_simple targets/custom/pseudoserver_simple.c

pseudoclient_simple: clean
	gcc -o targets/custom/pseudoclient_simple targets/custom/pseudoclient_simple.c

pseudotest_complex: pseudoserver_complex pseudoclient_complex

pseudoserver_complex: clean
	gcc -o targets/custom/pseudoserver_complex targets/custom/pseudoserver_complex.c

pseudoclient_complex: clean
	gcc -o targets/custom/pseudoclient_complex targets/custom/pseudoclient_complex.c

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true

./targets/custom/echo_loop: ./targets/custom/echo_loop.c
	gcc -O3 -o targets/custom/echo_loop $<

echo_loop: ./targets/custom/echo_loop