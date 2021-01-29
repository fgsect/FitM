#!/usr/bin/env python3

"""
This script outputs a whole connection, from beginning to end,
If you want, you can add `-v` to get delimeters, like
>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>NEXt>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
"""
# fitm-gen22-state0/envfile | grep INPUT_FILENAME
# INPUT_FILENAME=/path/to/FitM/saved-states/fitm-gen20-state3/out/main/queue/id:000269,src:000248,time:104360,op:havoc,rep:2

import os
import sys

NEXT = (
    ">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>NEXT>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>"
)
#ENVFILE = "envfile"
#IF_TOK = "INPUT_FILENAME="
PREV_INPUT_FILE = "prev_input"
PREV_INPUT_PATH = "prev_input_path"



if len(sys.argv) < 2:
    raise Exception("Usage: [-v] ./path/to/fitm-genX-stateY")

def faux_print(*args, **kwargs):
    pass

# -r => raw message, don't print information.
if len(sys.argv) > 2 and sys.argv[1] == "-v":
    current_state = sys.argv[2]
    # to silence the linter
    print = print
else:
    current_state = sys.argv[1]
    print = faux_print

connection_files = []

if not os.path.exists(current_state):
    raise Exception(f"Could not open initial state {current_state}, make sure you have the proper access rights!")

# Walk backwards though the linked file list
while current_state:
    try:
        prev_file = os.path.join(current_state, PREV_INPUT_FILE)
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

stdout = os.fdopen(sys.stdout.fileno(), "wb")

for con_file in connection_files:
    print(NEXT)
    print(con_file)

    try:
        with open(con_file, "rb") as f:
            content = (
                f.read()
            )  # errors='surrogateescape')#.decode("utf-8",errors='surrogatepass')
            print(f"DBG: len={len(content)}")
    except Exception as ex:
        content = b""
        print(f"File missing (cmin killed it?)")  # {ex}")

    print(NEXT)
    stdout.write(content)  # .encode("utf-8", errors="surrogateescape"))
    stdout.flush()
    print("")
