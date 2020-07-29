# Testing

If in doubt, check the Makefile. If there is nothing in the makefile just compile the `whatever_test.c` and run it.
Rust tests are run with: `cargo test`

# self-dump test

PoC to play with criu's RPC dump functionality. Practically the same as criu's `test-c.c`.  
Prints counter 10x. Dumps after 2 counts. Then stays dead until restored.
```sh
# Start criu server with:  
sudo ./criu/criu service -v4 --address /tmp/criu_service.socket --images-dir /tmp/criu_snapshot

# Build test with:
make clean && make CRIUPATH=/home/user/repos

# In extra terminal:  
watch cat log.txt

# Run self-dump:  
setsid stdbuf -oL ./self-dump < /dev/null &> log.txt

# Restore: 
sudo ./criu/criu restore -d -vvv -o restore.log --images-dir /tmp/criu_snapshot && echo OK
```