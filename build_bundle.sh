#/usr/bin/bash

status=0
(cd bootloader; cargo build); status=$(($status + $?))
(cd kernel; cargo build --target x86_64-unknown-none -Zbuild-std ); status=$(($status + $?))
(cd embed; cargo build); status=$(($status + $?))
target/debug/embed \
    target/x86_64-unknown-uefi/debug/bootloader.efi \
    target/x86_64-unknown-none/debug/kernel \
    target/bundle.efi; \
    status=$(($status + $?))
exit $status
