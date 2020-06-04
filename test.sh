#!/usr/bin/sh
# Make was annoying with errors/env and I was too tired for learning it atm
clean(){
  rm -rf envfile
  unset LETS_DO_THE_TIMEWARP_AGAIN
  unset TESTENV
  unset AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES
  unset AFL_SKIP_CPUFREQ
  unset AFL_DEBUG_CHILD_OUTPUT

  rm -rf /tmp/criu_snapshot/
  rm -rf /tmp/log
}

test_restore(){
  export LETS_DO_THE_TIMEWARP_AGAIN=1
  export AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1
  export AFL_SKIP_CPUFREQ=1
  export AFL_DEBUG_CHILD_OUTPUT=1
  mkdir -p /tmp/criu_snapshot
  # This throws a weird error(?) but seems to work:
  # test.sh: line 15: 608344 Killed    setsid stdbuf -oL AFLplusplus/afl-qemu-trace test/forkserver_test < /dev/null &> /tmp/log
  setsid stdbuf -oL AFLplusplus/afl-qemu-trace test/forkserver_test < out/.cur_input &> /tmp/log && echo "Initial snap successful"
  unset LETS_DO_THE_TIMEWARP_AGAIN
  sudo rm -f out/* &> /dev/null || echo "rm failed"
  mkdir "in" "out" &> /dev/null || echo "mkdir failed"
  echo "RI" > "in/foobar"
  sudo -E AFLplusplus/afl-fuzz -i in -o out -m none -r /tmp/criu_snapshot -- sh restore.sh
}

clean
test_restore
