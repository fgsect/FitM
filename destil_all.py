#!/usr/bin/env python3
import sys
import os
from destil_connection import destil
from multiprocessing.pool import ThreadPool

KILL_PILL = "lol u dead"


def main(saved_states, out_dir):

    os.mkdir(out_dir)

    if not saved_states.strip().replace("/", "").endswith("saved-states"):
        raise Exception("No valid a saved state folder given")

    p = ThreadPool()

    for state_dir in os.listdir(saved_states):
        print(f"Found state_dir {state_dir}")
        if "fitm-" in state_dir:
            p.apply_async(
                destil,
                (
                    os.path.join(saved_states, state_dir),
                    os.path.join(out_dir, state_dir),
                ),
            )
    p.close()
    p.join()


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
