#!/usr/bin/env bash
set -euo pipefail
source .env
source .secret.env

FLAVOR=${FLAVOR:-debug}

echo "Running docker/run-coord.sh from $(pwd)"
echo "FLAVOR=$FLAVOR, ARCH=$ARCH"
echo "Current directory contents:\
$(ls -l)"

for var in DITTO_APP_ID DITTO_PG_TOKEN; do
    if [[ ! -v $var ]]; then
        echo "Error: $var is not set"
        exit 1
    fi
done

echo "Copying libdittoffi.so to /lib"
find $ARCH/$FLAVOR -name libdittoffi.so \
    -exec cp {} /lib \;

echo "ENV: "; env
if [ "$FLAVOR" = "debug" ]; then
    set -x
    export RUST_BACKTRACE=1
fi

set -x
uname -a
CBIN="./$ARCH/$FLAVOR/cmesh-coordinator"
file $CBIN
$CBIN $@
set +x

echo "Finished docker/run-coord.sh"
sleep 2
