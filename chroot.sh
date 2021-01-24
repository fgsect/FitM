# Mount Kernel Virtual File Systems
TARGETDIR="/tmp/FitM"
mkdir -p $TARGETDIR/proc
mkdir -p $TARGETDIR/sys
mkdir -p $TARGETDIR/dev
mkdir -p $TARGETDIR/shm
mkdir -p $TARGETDIR/pts
mkdir -p $TARGETDIR/etc
mkdir -p $TARGETDIR/bin
mkdir -p $TARGETDIR/lib
mount --bind /bin $TARGETDIR/bin
mount --bind /lib $TARGETDIR/lib
mount -t proc proc $TARGETDIR/proc
mount -t sysfs sysfs $TARGETDIR/sys
mount -t devtmpfs devtmpfs $TARGETDIR/dev
mount -t tmpfs tmpfs $TARGETDIR/dev/shm
mount -t devpts devpts $TARGETDIR/dev/pts

# Copy /etc/hosts
/bin/cp -f /etc/hosts $TARGETDIR/etc/

# Copy /etc/resolv.conf
/bin/cp -f /etc/resolv.conf $TARGETDIR/etc/resolv.conf

# Link /etc/mtab
chroot $TARGETDIR rm /etc/mtab 2> /dev/null
chroot $TARGETDIR ln -s /proc/mounts /etc/mtab
