#/usr/bin/bash

(cd kernel && cargo check --message-format=json)
(cd inline_debug_info && cargo check --message-format=json)
