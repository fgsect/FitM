run:
	mkdir -p /tmp/criu_snapshot
	cc -g -Werror -Wall -I.   -c -o self-dump.o self-dump.c
	cc -g -Werror -Wall -I.   -c -o rpc.pb-c.o rpc.pb-c.c
	cc self-dump.o rpc.pb-c.o  -lprotobuf-c -o self-dump

clean:
	rm -rf self-dump self-dump.o rpc.pb-c.o rpc.proto

# Invoke with: make symlink CRIUPATH=/home/hirnheiner/repos
symlink:
	ln -s $(CRIUPATH)/criu/images/rpc.proto