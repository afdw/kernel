#/usr/bin/bash

(cd bootloader && cargo clippy)
(cd kernel && cargo clippy)
(cd embed && cargo clippy)
