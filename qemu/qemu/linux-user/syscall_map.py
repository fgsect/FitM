#!/usr/bin/env python3

"""create a header file for FITM syscall trace"""

print("static char *syscall_str[] = {")

with open("syscall_nr.h") as f:
    for line in f:
        targetns = line.split("TARGET_NR_")
        if len(targetns) > 1:
            print('    "' + targetns[1].split(" ", 1)[0].split("\t", 1)[0] + '",')

print("}")