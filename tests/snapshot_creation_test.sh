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
  rm -rf test-state
}

create_snap(){
  export LETS_DO_THE_TIMEWARP_AGAIN=1
  export AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1
  export AFL_SKIP_CPUFREQ=1
  export AFL_DEBUG_CHILD_OUTPUT=1
  export AFL_SKIP_BIN_CHECK=1
  export INPUT_FILENAME="./input_file"

  old_pwd=$PWD
  state_dir=$(pwd)/test-state
  export CRIU_SNAPSHOT_DIR=$state_dir/snapshot
  mkdir -p $CRIU_SNAPSHOT_DIR
  cd $state_dir
  mkdir fd
  touch stderr
  touch stdout
  touch input_file
  setsid stdbuf -oL ../../AFLplusplus/afl-qemu-trace ../snapshot_creation < /dev/null &> stdout
  cd ..
}


backup_snap(){
  sudo rm -rf /tmp/test
  sudo cp -r $state_dir /tmp/test
}

restore(){
#  unset LETS_DO_THE_TIMEWARP_AGAIN
  export CRIU_SNAPSHOT_DIR=$state_dir/snapshot2
  mkdir -p $CRIU_SNAPSHOT_DIR
  env > envfile
  sudo criu restore -d -vvv -o ./restore.log --images-dir ./test-state/snapshot && echo 'OK'
}

clean
create_snap
restore
# This sleep is needed for criu as otherwise criu will try to reuse the PID that the previous process just used
# but the PID is not freed up by the previous process yet.
sleep 0.1
#truncate --size=100 test-state/stdout
sudo criu restore -d -vvv -o ./restore.log --images-dir ./test-state/snapshot2 && echo 'OK'