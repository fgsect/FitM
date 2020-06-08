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
  export AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1
  export AFL_SKIP_CPUFREQ=1
  export AFL_DEBUG_CHILD_OUTPUT=1

  old_pwd=$PWD
  state_dir=$(pwd)/states/fitm-c0s0

  unset LETS_DO_THE_TIMEWARP_AGAIN
  sudo -E AFLplusplus/afl-fuzz -i $state_dir/in -o $state_dir/out -m none -r states/test -- sh restore.sh
}

# clean
test_restore
