#!/usr/bin/env bash

set -euo pipefail

TARGET_DIR="$1"
KMOD="${2:-}"

# Auto-detect kernel module path if not provided
if [ -z "$KMOD" ]; then
    KVER=$(uname -r)
    KMOD="/run/current-system/kernel-modules/lib/modules/${KVER}"
fi

# Resolve symlinks to get the actual path
KMOD=$(readlink -f "$KMOD")

if [ ! -d "$KMOD" ]; then
    echo "ERROR: Kernel modules not found at ${KMOD}" >&2
    exit 1
fi

# Copy required kernel modules for 9p root filesystem.
# Modules are kept as .ko.xz — the bootstrap code decompresses them at load
# time via lzma-rs into a memfd before calling finit_module(2).
mkdir -p "${TARGET_DIR}/lib/modules"

# Transform modules.dep: keep path structure but ensure paths match where we copy modules
# The modules will be stored with paths like kernel/fs/9p/9p.ko.xz
cp "${KMOD}/modules.dep" "${TARGET_DIR}/lib/modules/modules.dep"

copy_module() {
    local src="$1"
    # Resolve symlinks in source path
    src=$(readlink -f "$src")
    # Keep the full path relative to kernel/ e.g., kernel/fs/9p/9p.ko.xz
    local dst="${TARGET_DIR}/lib/modules/${src#"${KMOD}/"}"
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
}

# Module dependency order (from modprobe --show-depends):
# netfs (base for 9p)
copy_module "${KMOD}/kernel/fs/netfs/netfs.ko.xz"

# virtio core (for 9pnet_virtio transport)
copy_module "${KMOD}/kernel/drivers/virtio/virtio.ko.xz"
copy_module "${KMOD}/kernel/drivers/virtio/virtio_ring.ko.xz"
copy_module "${KMOD}/kernel/drivers/virtio/virtio_pci.ko.xz"
copy_module "${KMOD}/kernel/drivers/virtio/virtio_pci_modern_dev.ko.xz"
copy_module "${KMOD}/kernel/drivers/virtio/virtio_pci_legacy_dev.ko.xz"

# 9p network layer
copy_module "${KMOD}/kernel/net/9p/9pnet.ko.xz"
copy_module "${KMOD}/kernel/net/9p/9pnet_virtio.ko.xz"
copy_module "${KMOD}/kernel/net/9p/9pnet_fd.ko.xz"

# 9p filesystem
copy_module "${KMOD}/kernel/fs/9p/9p.ko.xz"

# virtio-net (for user-mode networking / NAT)
copy_module "${KMOD}/kernel/net/core/failover.ko.xz"
copy_module "${KMOD}/kernel/drivers/net/net_failover.ko.xz"
copy_module "${KMOD}/kernel/drivers/net/virtio_net.ko.xz"

# af_packet (required for DHCP/udhcpc)
copy_module "${KMOD}/kernel/net/packet/af_packet.ko.xz"

# Pack the initramfs
cd "$TARGET_DIR" && find . | cpio -o -H newc | gzip >../initramfs-minimal.cpio.gz
