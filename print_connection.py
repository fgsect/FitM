#!/usr/bin/env python3

"""
This script outputs a whole connection, from beginning to end, with the delimeters
>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>NEXt>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
"""
# fitm-gen22-state0/envfile | grep INPUT_FILENAME
# INPUT_FILENAME=/path/to/FitM/saved-states/fitm-gen20-state3/out/main/queue/id:000269,src:000248,time:104360,op:havoc,rep:2

import os
import sys

if len(sys.argv) < 2:
    raise Exception("Usage: ./path/to/fitm-genX-stateY")

current_state = sys.argv[1]

connection_files = []

NEXT = (
    ">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>NEXT>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>"
)
ENVFILE = "envfile"
IF_TOK = "INPUT_FILENAME="

# Walk backwards though the linked file list
while current_state:
    try:
        with open(os.path.join(current_state, ENVFILE)) as f:
            for line in f:
                if line.startswith(IF_TOK):
                    prev = line[len(IF_TOK) :].strip()
                    connection_files.insert(0, prev)
                    current_state = os.path.dirname(prev)
                    for i in range(3):
                        # get out ouf /out/main
                        current_state = os.path.dirname(current_state)

    except Exception as ex:
        # print(f"Initial handling finished: {ex}")
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
