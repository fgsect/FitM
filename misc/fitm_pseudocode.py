def afl_fuzz(snap, inputs, run_time):
    """
    Runs afl fuzz
    """
    fuzz_result = some_magic(snap, inputs, run_time)
    return fuzz_result


def afl_cmin(snap, inputs):
    """
    Minimizes the inputs
    """
    minimized_inputs = some_magic(snap, inputs)
    return minimized_inputs


def process_stage(current_snaps, current_inputs, next_inputs, nextnext_snaps):
    """
    Run afl_fuzz for each snapshot with all inputs for the current gen
    @param current_snaps: list of snapshots for this stage
    @param current_inputs: list of inputs for this stage
    @param next_inputs: (out) list of inputs for the next stage (client->server//server->client)
    @param nextnext_snaps: (out) list of snapshots based off of this base snap (client->client, server->server)
    @return: False, if we didn't advance to the next generation (no more output)
    """
    for snap in current_snaps:
        # Not necessary but could be nice: ignore inputs that don't yield new cov for fuzzing
        inputs = afl_cmin(snap, current_inputs)

        fuzz_result = afl_fuzz(snap, inputs)
        # Post-processing: prune queue entries that don't yield new cov.
        minimized_queue = afl_cmin(snap, fuzz_result.queue)

        # Get all outputs for the nth client run
        for queue_entry in minimized_queue:
            output = snap.restore().input(queue_entry).run_to_recv().output()
            if output:
                next_gen_valid = True
                next_inputs.append(output)

        # Get all snapshots for the n+1 server run (later)
        # This could also be done at a later time.
        for queue_entry in minimized_queue:
            nextnext_snaps.append(
                snap.restore().input(queue_entry).run_to_recv().snapshot()
            )


def gen_is_client(gen_id):
    """
    We begin fuzzing with the server (gen == 0), then client (gen == 1), etc.
    So every odd numbered is a client.
    """
    return (gen_id % 2) == 1


def main():

    client_binary = "testclient"
    server_binary = "testserver"

    snapshots = []
    generation_inputs = []

    # The initial server is right after the initial recv
    fitm_server = server_binary.run_to_recv().snapshot()
    snapshots[0] = [fitm_server]

    # The initial client is the snapshot right after the inital recv (after n sends)
    fitm_client = client_binary.run_to_recv().snapshot()
    snapshots[1] = [fitm_client]

    # The initial server input is the initial client output, of all sends before the recv
    initial_input = client_binary.run_to_recv().output()
    if not initial_input:
        raise Exception(
            "Uh oh, client misbehaved! No initial input for server generated!"
        )
    generation_inputs[0] = [initial_input]

    current_gen = 0

    while True:

        if gen_is_client(current_gen):
            print(f"Fuzzing client (gen {current_gen})")
        else:
            print(f"Fuzzing server (gen {current_gen})")

        # server -> client or client -> server
        next_gen = current_gen + 1
        # client -> client or server -> server
        nextnext_gen = current_gen + 2

        # make sure the next stage inputs list exist
        if not hasattr(generation_inputs, next_gen):
            generation_inputs[next_gen] = []
        # make sure the nextnext stage snapshot exists
        if not hasattr(snapshots, nextnext_gen):
            generation_inputs[nextnext_gen] = []

        process_stage(
            snapshots[current_gen],
            generation_inputs[current_gen],
            generation_inputs[next_gen],
            snapshots[nextnext_gen],
        )

        # Continue with the next gen (server->client or vice versa)
        current_gen += 1

        # ... Unless we did not create any inputs for the next gen (our sends were all empty or we crashed...
        # In that case: restart fuzzin from gen 0. :)
        if len(generation_inputs[current_gen]) == 0:
            current_gen = 0
