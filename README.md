# FitM
Fuzzer in the Middle

## Building

**NOTE**:   
We use criu's `crit` tool to parse snapshot images. There is a bug in the tool that needs fixing.
Replace L360 in `criu/lib/py/images/pb2dict.py` with the following code:  
`d[field.name] = d_val.decode() if type(d_val) == bytes else d_val`


`make build` can be used to run our build script to build our custom qemu patches

On Ubuntu it was necessary to install the following package to execute criu's crit.
`sudo apt install python-ipaddress`

## Testing

`sh test.sh` can be used to correctly dump a qemu process outside of AFL and then try to restore it inside AFL.
The process output is found in `/tmp/log`. The restore log is found in `/tmp/criu_snapshot/restore.log`.
You may need to disable line 1206 + 1207 in `AFLplusplus/src/afl-fuzz.c`. I think it breaks restoring prematurely but
I'm not sure anymore.

### Forkserver test

When testing the forkserver starting in `do_recv` remember the following things:
- Don't have a `envfile` in your current working dir
- Don't have `LETS_DO_THE_TIMEWARP_AGAIN` set
- Have the following lines disabled in your `syscall.c` and rebuild after disabling
    ```c
    if (!getenv_from_file("LETS_DO_THE_TIMEWARP_AGAIN")) {
        exit(0);
    }
    sent = false; // After restore, we'll await the next sent before criuin' again
    do_criu();
    ```

```
gcc -o forkserver_test forkserver_test.c
```

To execute AFL
```
AFL_DEBUG_CHILD_OUTPUT=1 LETS_DO_THE_TIMEWARP_AGAIN=1 ../AFLplusplus/afl-fuzz -i in -o out -Q -m 10000 -- ./forkserver_test
```

## Execution

Because criu needs root privs and the restored process needs root privs to access the shared memory from AFL everything has to be run as root.
So please execute everything with `sudo`.

## Qemu folder

Inlucdes all necessary `.h` files and similar from AFL++ to build a compatible qemu binary.
This allows us to no write patch files while developing qemu.

Build with: `cd qemu/qemu/ && ./build-for-afl.sh`

## CRIU Build Dependencies
### Ubuntu

To compile and run CRIU, these packages need to be installed:
`sudo apt install protobuf-c-compiler libprotobuf-c-dev libnet-dev python3-protobuf python3-yaml`

## Git Submodule

A few words about submodules if you are not familiar with it. AFL++ is included as a submodule for easier development.

### TLDR

Run:
```sh
git config submodule.recurse true # Run commands supporting it with --recurse-submodules
git config push.recurseSubmodules on-demand # Push submodule changes automatically if possible
git clone --recurse-submodules <FitM-URL> # Update/init submodules while cloning
```

## Slightly longer
- Read [this](https://git-scm.com/book/en/v2/Git-Tools-Submodules) for more info
- Git verion >= 2.14 makes things easier
- `git diff` on submodule folder will only show the commit tracked by the FitM repo, not any changes that may be in the folder
- Clone with the follwing option: `git clone --recurse-submodules <FitM-URL>`. Without this option only an empty folder will be created for each submodule. These folders can be populated with the following commands:
  ```sh
    git submodule update --init
  ```
  You always need to run `git submodule update` once you ran `git pull`. You can also set `git config submodule.recurse true` in order to always update submodules when pulling.
- Run `git config push.recurseSubmodules on-demand` in order to configure git to automatically push submodule changes before push main project changes
- Altenatively use aliases:
  ```sh
    git config alias.sdiff '!'"git diff && git submodule foreach 'git diff'"
    git config alias.spush 'push --recurse-submodules=on-demand'
    git config alias.supdate 'submodule update --remote --merge'
    ```
- If you didn't set the recurse option above, witching branches in the main modules fucks up submodules if the submodules state is different on each branch

## Core concept

```
    let set = btree.new()
    let client = afl.new()
    client.init_run()
    let server = afl.new()
    server.init_run()


    if client.sent_first()
        let cur_run = server
    else
        let cur_run = client

    loop {
        cur_run.fuzz_run()

        cur_run.gen_afl_maps()

        // new_files = Vec<files>

        for file in state_dir/out/showmaps/ {
            !set.contains(file.content) {
                // set.append(file.content)
                afl.new()
            }
        }

        // TODO: For later
        // if new_files.len() > 0 && hit_recv {
        //     new_run = afl.run()
        // }

        cur_run = queue.dequeue()
    }
```
