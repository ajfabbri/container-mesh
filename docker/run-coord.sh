#!/usr/bin/env bash
set -euo pipefail

echo "Running docker/run-coord.sh from $(pwd)"
echo "Current directory contents:\
$(ls -l)"

echo "Copying libdittoffi.so to /lib"
find $COORD_ARCH/release -name libdittoffi.so \
    -exec cp {} /lib \;

set -x
uname -a
CBIN="./$COORD_ARCH/release/cmesh-coordinator"
file $CBIN
$CBIN $@
set +x

echo "Finished docker/run-coord.sh"
sleep 2
