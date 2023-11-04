#!/usr/bin/env bash
set -euo pipefail

FLAVOR=${FLAVOR:-debug}

echo "Running docker/run-peer.sh from $(pwd)"
echo "FLAVOR=$FLAVOR, ARCH=$ARCH"
echo "Current directory contents:\
$(ls -l)"

for var in DITTO_APP_ID DITTO_PG_TOKEN DITTO_LICENSE; do
    if [[ ! -v $var ]]; then
        echo "Error: $var is not set"
        exit 1
    fi
done

echo "Copying libdittoffi.so to /lib"
find $ARCH/$FLAVOR -name libdittoffi.so \
    -exec cp {} /lib \;

echo "---> env"
env
set -x
if [ "$FLAVOR" = "debug" ]; then
    export RUST_BACKTRACE=1
fi

uname -a
PBIN="./$ARCH/$FLAVOR/cmesh-peer"
file $PBIN
$PBIN $@
set +x

echo "Finished docker/run-peer.sh"
sleep 2

