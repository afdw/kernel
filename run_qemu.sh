#/usr/bin/bash

qemu-system-x86_64 \
    -enable-kvm \
    -machine q35 \
    -serial stdio \
    -m 512m \
    -device virtio-gpu-pci -vga none \
    -drive if=none,format=qcow2,file=snapshots.qcow2 \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_VARS.fd \
    -drive if=virtio,format=raw,readonly=on,media=disk,file=image.img \
    -loadvm vm-20230525130451
