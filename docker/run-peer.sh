#!/usr/bin/env bash
set -euo pipefail

echo "Running docker/run-peer.sh from $(pwd)"
echo "Current directory contents:\
$(ls -l)"

for var in DITTO_APP_ID DITTO_PG_TOKEN; do
    if [[ ! -v $var ]]; then
        echo "Error: $var is not set"
        exit 1
    fi
done

echo "Copying libdittoffi.so to /lib"
find $PEER_ARCH/release -name libdittoffi.so \
    -exec cp {} /lib \;

set -x
uname -a
PBIN="./$PEER_ARCH/release/cmesh-peer"
file $PBIN
$PBIN $@
set +x

echo "Finished docker/run-peer.sh"
sleep 2

