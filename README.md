# Getting It

This repo uses submodules :(.  
Clone the repo with `--recurse-submodules` to initialize and download all submodules.
Alternatively, you can run `make subinit` if you did not clone with above option.

# Building

```
vagrant up
vagrant ssh
cd /vagrant
make
```

If you don't want to use the provided Vagrantfile you can read the provided `provision.sh` script to understand then necessary dependencies. In general you will need packages to build [QEMU](https://github.com/AFLplusplus/qemuafl/blob/master/README.rst) and [Criu](https://criu.org/Installation#Installing_build_dependencies). Additionally, you will need a stable [rust toolchain](https://www.rust-lang.org/tools/install).

# Special Files
## fitm-args.json

This file is used to configure FitM and includes a dictionary with various keys. Following is a description of each key:

- "client": path to the binary that should be gen0.
- "client_args": command-line arguments for the client binary.
- "client_envs": environment variables that will be available to the client binary.
- "client_files": currently unused.
- "server": path to the binary that should be gen1.
- "server_args": command-line arguments for the server binary.
- "server_envs": environment variables that will be available to the server binary.
- "server_files": currently unused.
- "run_time": time spent fuzzing each generation in seconds.
- "server_only": boolean to indicate that we only want to fuzz the server. The client is only fuzzed for 100ms and it's output is disregarded. 

## fitm-state.json

A JSON file used to save state information from previous runs. This allows us to abort fuzzing at any point, introduce changes and then reuse the accumulated states in `./saved-states`. The file holds a serialized form of the `generation_snaps` variable. This variable holds all list of generations, each generation being another list of `FITMSnapshot` objects (`[[gen0_snap0, gen0_snap1, .., gen0_snapN], [gen1_snap0, .. gen1_snapN], .., [genN_snap0, .., genN_snapN]]`).

# Running 

FITM_ARGS=/path/to/fitm-args.json make run

# Cite / More Information

We wrote a paper about this tool. You can get a first idea of how the fuzzer works there.
[link](...).

