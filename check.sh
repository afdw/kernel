#/usr/bin/bash

(cd bootloader && cargo check --message-format=json)
(cd kernel && cargo check --message-format=json)
(cd embed && cargo check --message-format=json)
