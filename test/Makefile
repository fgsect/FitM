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
	pseudoserver pseudoclient

test_getenv: clean
	gcc -o getenv_from_file_test getenv_from_file_test.c
	./getenv_from_file_test

test_pipe: clean
	gcc -o pipe_test pipe_test.c
	setsid stdbuf -oL ../AFLplusplus/afl-qemu-trace ./pipe_test < /dev/null &> pipe_test.out

pseudoserver: clean
	gcc -o pseudoserver pseudoserver.c
	../AFLplusplus/afl-qemu-trace ./pseudoserver

pseudoclient: clean
	gcc -o pseudoclient pseudoclient.c
	../AFLplusplus/afl-qemu-trace ./pseudoclient

test_criupipe: clean
	gcc -o pipe pipe.c
	./pipe


# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true
