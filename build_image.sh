#/usr/bin/bash

rm -f image.img
truncate -s 128M image.img
mkfs.fat image.img > /dev/null
mkdir mnt
sudo mount image.img mnt
sudo mkdir -p mnt/efi/boot
sudo cp target/x86_64-unknown-uefi/debug/kernel.efi mnt/efi/boot/bootx64.efi
sudo umount mnt
rmdir mnt
