selfdump: symlink
	mkdir -p /tmp/criu_snapshot
	protoc-c --proto_path=. --c_out=. rpc.proto
	cc -g -Wall -I.   -c -o self-dump.o self-dump.c
	cc -g -Wall -I.   -c -o rpc.pb-c.o rpc.pb-c.c
	cc self-dump.o rpc.pb-c.o  -lprotobuf-c -o self-dump

syscall:
	gcc -o syscall syscall_test.c

clean:
	rm -rf self-dump self-dump.o rpc.pb-c.o rpc.pb-c.c rpc.pb-c.h rpc.proto syscall getenv_from_file_test

test: clean
	gcc -o getenv_from_file_test getenv_from_file_test.c
	./getenv_from_file_test

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos/criu
symlink:
	ln -s $(CRIUPATH)/images/rpc.proto || true