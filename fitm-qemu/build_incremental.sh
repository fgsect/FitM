#!/bin/sh
#
# Just build incremental. No extra checks.

if [ -n "$HOST" ]; then
  echo "[+] Configuring host architecture to $HOST..."
  CROSS_PREFIX=$HOST-
else
  CROSS_PREFIX=
fi

echo "[*] Configuring QEMU for $CPU_TARGET..."

ORIG_CPU_TARGET="$CPU_TARGET"

if [ "$ORIG_CPU_TARGET" = "" ]; then
  CPU_TARGET="`uname -m`"
  test "$CPU_TARGET" = "i686" && CPU_TARGET="i386"
  test "$CPU_TARGET" = "arm64v8" && CPU_TARGET="aarch64"
  case "$CPU_TARGET" in 
    *arm*)
      CPU_TARGET="arm"
      ;;
  esac
fi

echo "Building for CPU target $CPU_TARGET incrementally"

cd ./FitM-qemu
make || exit 1
echo "[+] copying to ../../fitm-qemu-trace"
cp -f "build/${CPU_TARGET}-linux-user/qemu-${CPU_TARGET}" "../../fitm-qemu-trace" || exit 1
echo "[+] done"

