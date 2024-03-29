# FitM, the Fuzzer in the Middle

<p align="center">
  <img width="460" height="300" src="https://user-images.githubusercontent.com/22647728/158073817-5ed845b2-46ea-4ce9-8ae3-4103b30653f6.gif">
</p>
  
FitM, the Fuzzer-in-the-Middle, is a AFL++-based coverage-guided fuzzer for stateful, binary-only client-server applications. 
It can be used in situations where you would normally turn to grammar-based fuzzers or start patching your target. With FitM you can explore the communication between client and server by fuzzing them at the same time.
It builds on top of [qemuafl](https://github.com/AFLplusplus/qemuafl) for emulation and [CRIU](https://criu.org/Main_Page) for userspace snapshots. No source code needed!

## How it works

The FitM tool uses [FitM-qemu](https://github.com/fgsect/FitM-qemu) for instrumentation.
FitM-qemu extends qemuafl with a network emulation layer on the syscall level.
With it, we can fuzz two targets (binary A & B) at the same time, usually a server and a client and schedule different snapshots of these processes ("generations") we collect while exploring the protocol.
Each generation represents a stage in the protocol's communication levels that the client-server pair speaks.
The fuzzer starts in generation 0 with binary A. This binary should produce some output without needing any input. 
Using the initial output of binary A as seed, FitM will fuzz binary B for a while.
Afterwards, FitM creates a set of new snapshots of B, generation 3, for later.
Next, the snapshot of generation 0 (binary A) is restored, seeded with generation 1's output and fuzzed (generation 2).
A snapshot is always created during a receive call that followed a send call until we fully explore the client-server interaction.
The below figure depicts this cycle.


<p align="center">
<img width="400" alt="Overview over the different stages of FitM, see paper" src="https://user-images.githubusercontent.com/297744/159170739-f8d8d551-e42f-4c76-ae62-902d44b86026.svg" align="center">
</p>

See [our paper](fitm.pdf) for technical explanations, benchmarks, and further details.
  
## Getting Started

This module uses submodules. Clone with `--recurse-submodules`.
  
Alternatively, run the following after cloning:  
```
git submodule init
git submodule update
```

## Building

```
vagrant up
vagrant ssh
cd /vagrant
make
```

If you don't want to use the provided Vagrantfile you can read the provided `provision.sh` script to understand the necessary dependencies. In general you will need packages to build [QEMU](https://github.com/AFLplusplus/qemuafl/blob/master/README.rst) and [Criu](https://criu.org/Installation#Installing_build_dependencies). Additionally, you will need a stable [rust toolchain](https://www.rust-lang.org/tools/install).

## Running 
Run this: `FITM_ARGS=config/fitm-args.ftp.json make run`

The fuzzer will create the folders `active-state`, `saved-states` and `cmin-tmp`. 
Whenever afl-cmin is used the inputs that should be fed into cmin are put into `cmin-tmp`.
`active-state` holds the necessary folder/files for FitM's operation and the restored snapshot's files.
The structure is as follows:

- `fd`: files that are used by the process. You will find current output here.
- `in`, `out`: afl's `in`/`out` folders.
- `next_snapshot`: populated during `create_next_snapshot()` with the files produced by criu. Renamed to snapshot and eventually copied to `saved-states`. 
- `out_postrun`: the content of the `out` folder after fuzzing.  
- `outputs`: folder with "persisted" outputs. Generally, output is written to files in the `fd` folder, but since those files (and the folder) need to be returned to the state they were in before restoring in order to snapshot the next state we collect outputs in an extra step `create_outputs()` and store them in the outputs folder.
- `snapshot`: serialized process data, i.e. the snapshot. The criu docs are helpful here.
- `envfile`: env for target process. Read by `getenv_from_file()` (see `./fitm-qemu/FitM-qemu/qemuafl/fitm.h`) in QEMU syscall translation layer.
- `pipes`: names of forkserver pipes. Needed to reconnect pipes in restored snapshot to pipes from forkserver. Done with the `--inherit-fd` argument in `./active-state/restore.sh`.
- `prev_input` / `prev_input_path`: input and path to input file that was used to generate current snapshot.
- `restore.log`: criu output of the snapshot restore process.
- `run-info`: serialized FITMSnapshot object for the active state. Helps to know where you are.
- `snapshot_map`: afl-map output for the snapshot with prev_input.
- `stdout`/`stderr`: stdout/err of the target process.
- `stdout-afl`/`stderr-afl`: stdout/err from the AFL process.

Each folder in `saved-states` represents one snapshot plus FitM-related files. Some of the files, e.g. `pipes`, related to each snapshot are specific to the snapshots state and thus have to be managed for every state.
In general, criu breaks very quickly if a process had a handle to a file while it ran, but the file changes (contents, path, size, metadata) in any way between snapshot and restore. 

Apart from the above three folder you will find the following temporary files in the repo's root folder after running FitM:

- `criu_stdout`/`criu_stderr`: stdout/err of the criu server process. To create snapshots each target process communicates with a separate criu process, the criu server. 
- `fdinfo`, `file`: Criu has a tool called "crit" that can be used to parse the binary files that are part of a snapshot folder. We use crit to parse the open files in the target process and attach them accordingly. The parsing code can be found in `create_restore.py`. This script create a bash script `restore.sh` based on the `restore.sh.tmp` template for each state. The `restore.sh` script is the target given to AFL when starting another fuzz run. The script will call `criu restore` and by using the [--restore-detached](https://criu.org/Tree_after_restore#Detached) flag we make sure that the target process ends up as a child of AFL after criu has exited.

## Special Files
### fitm-args.json

This file is used to configure FitM. The meaning of each key is as follows:

- `client`: path to the binary that should be gen0.
- `client_args`: command-line arguments for the client binary.
- `client_envs`: environment variables that will be available to the client binary.
- `client_files`: currently unused.
- `server`: path to the binary that should be gen1.
- `server_args`: command-line arguments for the server binary.
- `server_envs`: environment variables that will be available to the server binary.
- `server_files`: currently unused.
- `run_time`: time spent fuzzing each generation in seconds.
- `server_only`: boolean to indicate that we only want to fuzz the server. The client is only fuzzed for 100ms and it's output is disregarded. 

### fitm-state.json

A JSON file used to save state information from previous runs. This allows us to abort fuzzing at any point, introduce changes and then reuse the accumulated states in `./saved-states`. The file holds a serialized form of the `generation_snaps` variable. This variable holds a list of generations that need to be fuzzed, each generation being another list of `FITMSnapshot` objects (`[[gen0_snap0, gen0_snap1, .., gen0_snapN], [gen1_snap0, .. gen1_snapN], .., [genN_snap0, .., genN_snapN]]`).

## Debugging

When using FitM with a new target you will probably investigate weird behaviour sooner or later. 
You will generally want to check `./active-state/restore.log` and see the log end with a message similar to this one:
```
(00.052019) Running pre-resume scripts
(00.052043) Restore finished successfully. Tasks resumed.
(00.052062) Writing stats
(00.052705) Running post-resume scripts
```
At this point you know that the target was successfully restored by criu and any further fails come from the target misbehaving. 
You will want to check `./active-state/stdout` for the target's stdout, in case you are doing printf-debugging. 
When you set the `FITM_DEBUG` macro to 1 you will find a lot of debug prints there that might give you a first idea of where things are breaking.
Next, you can add `QEMU_STRACE` with a value of 1 to the `client_envs`/`server_envs` in your fitm-args.json to get strace output from QEMU. 
You will see the strace output in `./active-state/stderr`. These are the syscalls that the emulated target sends to emulation layer. 
By diffing the syscall traces of the target with and without FITM enabled you should be able to learn where things break (see `FITM_DISABLED` in `./fitm-qemu/FitM-qemu/linux-user/syscall.c`).

## Paper / Citing / More Information

<a href="fitm.pdf"> <img width="200" alt="The FitM paper" src="https://user-images.githubusercontent.com/297744/159168821-993d1f88-cc7d-48b0-a0e8-2522b035f789.png" align="right"> </a>

For further details, see [our paper](fitm.pdf) at BAR 2022.
  
To cite, use:
```bib
@InProceedings{fitm,
  title     = {FitM: Binary-Only Coverage-Guided Fuzzing for Stateful Network Protocols},
  author    = {Maier, Dominik and Bittner, Otto and Munier, Marc and Beier, Julian},
  booktitle = {Workshop on Binary Analysis Research (BAR), 2022},
  year      = 2022,
}
```

