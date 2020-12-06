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
	targets/pseudoclient targets/pseudoserver targets/snapshot_creation

test_getenv: clean
	gcc -o getenv_from_file_test getenv_from_file_test.c
	./getenv_from_file_test

test_pipe: clean
	gcc -o pipe_test pipe_test.c
	setsid stdbuf -oL ../AFLplusplus/afl-qemu-trace ./pipe_test < /dev/null &> pipe_test.out

snapshot_creation:
	gcc -o targets/snapshot_creation targets/snapshot_creation.c

echo_server:
	gcc -o targets/echo_server targets/echo_server.c

pseudotest_simple: pseudoserver_simple pseudoclient_simple

pseudoserver_simple: clean
	gcc -o targets/pseudoserver_simple targets/pseudoserver_simple.c

pseudoclient_simple: clean
	gcc -o targets/pseudoclient_simple targets/pseudoclient_simple.c

pseudotest_complex: pseudoserver_complex pseudoclient_complex

pseudoserver_complex: clean
	gcc -o targets/pseudoserver_complex targets/pseudoserver_complex.c

pseudoclient_complex: clean
	gcc -o targets/pseudoclient_complex targets/pseudoclient_complex.c

test_criupipe: clean
	gcc -o pipe pipe.c
	./pipe

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true
