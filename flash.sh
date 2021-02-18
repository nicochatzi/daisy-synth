#!/usr/bin/env bash
set -eo pipefail
set -x

cargo objcopy --release -- -O binary daisy.bin
dfu-util -a 0 -s 0x08000000:leave -D daisy.bin -d ,0483:df11