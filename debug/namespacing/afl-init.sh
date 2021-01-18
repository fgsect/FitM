#!/bin/sh

# Had the feeling that this had to be reinitialized in the namespace
echo core >/proc/sys/kernel/core_pattern
cd /sys/devices/system/cpu
echo performance | tee cpu*/cpufreq/scaling_governor