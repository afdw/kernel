#/usr/bin/bash

fast=0
export_root=0

while getopts "f" opt; do
  case "$opt" in
    f)
      fast=1
      ;;
    r)
      export_root=1
      ;;
  esac
done

((!$fast)) && rm -f image.img
((!$fast)) && truncate -s 128M image.img
((!$fast)) && sfdisk -q image.img <<EOF
label: gpt
start=-, size=16MiB, bootable, type=uefi
start=-, size=+, name=kernel_root, type=linux
EOF
mkdir mnt
loop_device=$(sudo losetup -f --show -P image.img)
((!$fast)) && sudo mkfs.fat ${loop_device}p1 > /dev/null
sudo mount ${loop_device}p1 mnt
sudo mkdir -p mnt/efi/boot
sudo cp target/x86_64-unknown-uefi/debug/kernel.efi mnt/efi/boot/bootx64.efi
sudo umount mnt
((!$fast)) && sudo mkfs.ext2 -q ${loop_device}p2
((!$fast)) && sudo mount ${loop_device}p2 mnt
((!$fast)) && sudo tee -a mnt/example <<< text > /dev/null
((!$fast)) && sudo umount mnt
((!$export_root)) && sudo dd if=${loop_device}p2 of=kernel_root.img bs=64K status=none
sudo losetup -D $loop_device
rmdir mnt
