#!/bin/bash

export LIBS='-lprotobuf-c -luuid'
export LDFLAGS="$LIBS"
export QEMU_LDFLAGS="$LIBS"
./configure --disable-system --enable-linux-user --disable-gtk --disable-sdl --disable-vnc \
            --enable-capstone=internal --target-list="x86_64-linux-user" --disable-bsd-user \
            --disable-guest-agent --disable-strip --disable-werror --disable-gcrypt \
            --disable-debug-info --disable-debug-tcg --disable-tcg-interpreter --enable-attr \
            --disable-brlapi --disable-linux-aio --disable-bzip2 --disable-bluez --disable-cap-ng \
            --disable-curl --disable-fdt --disable-glusterfs --disable-gnutls --disable-nettle \
            --disable-gtk --disable-rdma --disable-libiscsi --disable-vnc-jpeg --disable-lzo \
            --disable-curses --disable-libnfs --disable-numa --disable-opengl --disable-vnc-png \
            --disable-rbd --disable-vnc-sasl   --disable-sdl --disable-seccomp --disable-smartcard \
            --disable-snappy --disable-spice --disable-libssh2 --disable-libusb --disable-usb-redir \
            --disable-vde --disable-vhost-net --disable-virglrenderer --disable-virtfs --disable-vnc \
            --disable-vte --disable-xen --disable-xen-pci-passthrough --disable-xfsctl \
            --disable-system --disable-blobs --disable-tools --python=`which python3` \
	    --extra-ldflags="$LIBS"

make -j$(nproc) CLFAGS="$LIBS"
cp ./x86_64-linux-user/qemu-x86_64 ../../AFLplusplus/afl-qemu-trace
