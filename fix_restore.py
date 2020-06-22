#!/usr/bin/python
from sys import argv
from os import getcwd
from subprocess import call

import json

def main():
    if not argv[1]:
        return

    # execute from fitm/

    base_state = f"{getcwd}/active-state/{argv[1]}"[1:]
    cur_state = f"{getcwd}/active-state/{argv[2]}"[1:]

    lines = [x.strip() for open("./restore.sh.tmp", "r").readlines()]
    lines.append(f"    --inherit-fd[1]:{base_state}/stdout")
    lines.append(f"    --inherit-fd[2]:{base_state}/stderr")

    call(f"python3 crit decode -i /{base_state}/snapshot/files.img --pretty -o ./file".split())
    call(f"python3 crit decode -i /{base_state}/snapshot/fdinfo-2.img --pretty -o ./fdinfo".split())

    file_info = json.load(open("./files", "r"))
    fd_info = json.load(open("./fdinfo", "r"))

    files = filter(lambda x: "reg" in x.keys() and "/fd/" in x["reg"]["name"], file_info["entries"])
    files = map(lambda x: (x["id"], x["reg"]["name"]) , files)

    mapping = []

    for f in files:
        fd = list(filter(lambda x: x["id"] == f[0], fd_info["entries"]))[0]
        mapping.append((fd["id"], f[1]))

    for m in mapping:
        lines.append(f"    --inherit-fd[{m[0]}]:{m[1][1:]}")

    lines.append("    && echo 'OK'")

    open("./restore.sh", "w").write("\n".joint(lines))

if __name__ == "__main__":
    main()