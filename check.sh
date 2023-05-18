#/usr/bin/bash

cargo check --message-format=json
(cd inline_debug_info && cargo check --message-format=json)
