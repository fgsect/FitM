run: symlink
	mkdir -p /tmp/criu_snapshot
	protoc-c --proto_path=. --c_out=. rpc.proto
	cc -g -Wall -I.   -c -o self-dump.o self-dump.c
	cc -g -Wall -I.   -c -o rpc.pb-c.o rpc.pb-c.c
	cc self-dump.o rpc.pb-c.o  -lprotobuf-c -o self-dump

clean:
	rm -rf self-dump self-dump.o rpc.pb-c.o rpc.pb-c.c rpc.pb-c.h rpc.proto

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos
symlink:
	ln -s $(CRIUPATH)/criu/images/rpc.proto