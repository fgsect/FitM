# FitM
Fuzzer in the Middle

# Qemu folder

Inlucdes all necessary `.h` files and similar from AFL++ to build a compatible qemu binary. 
This allows us to no write patch files while developing qemu.  

Build with: `cd qemu/qemu/ && ./build-for-afl.sh`

# Git Submodule

A few words about submodules if you are not familiar with it. AFL++ is included as a submodule for easier development.

## TLDR

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
