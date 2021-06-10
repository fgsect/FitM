#!/usr/bin/env python3

"""
This script destils a whole connection, from beginning to end, to a folder
"""
# fitm-gen22-state0/envfile | grep INPUT_FILENAME
# INPUT_FILENAME=/path/to/FitM/saved-states/fitm-gen20-state3/out/main/queue/id:000269,src:000248,time:104360,op:havoc,rep:2

import os
import shutil
import sys
from shutil import copyfile

NEXT = (
    ">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>NEXT>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>"
)
# ENVFILE = "envfile"
# IF_TOK = "INPUT_FILENAME="
PREV_INPUT_FILE = "prev_input"
PREV_INPUT_PATH = "prev_input_path"


def destil(state_dir, out_dir, destilfile_dir=None):

    current_state = state_dir
    connection_files = []

    try:
        os.mkdir(out_dir)
    except Exception as ex:
        print("Failed to create out_dir: {}", ex)

    if destilfile_dir:
        try:
            os.mkdir(destilfile_dir)
        except Exception as ex:
            print("destilfile_dir already existed ({})", ex)

    # Walk backwards though the linked file list
    while current_state:
        try:
            prev_file = os.path.join(current_state, PREV_INPUT_PATH)
            if not os.path.exists(prev_file):
                print(f"finished in gen {prev_file}")
                break

            connection_files.insert(0, os.path.join(current_state, PREV_INPUT_FILE))

            with open(prev_file) as f:
                prev = f.read()

            # "cd" out ouf /out/main/filename
            current_state = os.path.dirname(prev)
            for i in range(3):
                current_state = os.path.dirname(current_state)

        except Exception as ex:
            print(f"Initial handling finished: {ex} ({current_state})")
            current_state = None

    for (i, con_file) in enumerate(connection_files):
        print("Copying", con_file, i)
        copyfile(con_file, f"{out_dir}/{i}")

    if destilfile_dir:
        destil_out = os.path.join(destilfile_dir, os.path.split(state_dir)[-1])
        print(f"Writing out dir {out_dir} to destil dir {destil_out}")
        with open(destil_out, "w") as f:
            f.write(os.path.abspath(out_dir))


if __name__ == "__main__":

    if len(sys.argv) != 3:
        raise Exception("Usage: ./script.py <./path/to/fitm-genX-stateY> <outdir>")

    in_state = sys.argv[1]
    out_dir = sys.argv[2]

    if not os.path.exists(in_state):
        raise Exception(
            f"Could not open initial state {in_state}, make sure you have the proper access rights!"
        )
    destil(state_dir=in_state, out_dir=out_dir)
