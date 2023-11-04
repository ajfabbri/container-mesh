#!/usr/bin/env bash
set -euo pipefail

echo "Running docker/run-peer.sh from $(pwd)"
echo "Current directory contents:\
$(ls -l)"

echo "Copying libdittoffi.so to /lib"
find $PEER_ARCH/release -name libdittoffi.so \
    -exec cp {} /lib \;

uname -a
PBIN="./$PEER_ARCH/release/cmesh-coordinator"
file $PBIN
$PBIN $@
set +x

echo "Finished docker/run-peer.sh"
sleep 2

