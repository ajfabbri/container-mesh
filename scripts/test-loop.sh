#!/usr/bin/env bash
set -eu -o pipefail -x
cd "$(dirname "${BASH_SOURCE[0]}")/.."
source .env

export ARCH=${ARCH:-x86_64-unknown-linux-gnu}
export FLAVOR=release
export RUST_LOG=warning

# Automated iterations of container mesh tests.

SCALE=${SCALE:-20}
ITERATIONS=${ITERATIONS:-2}
OUT_DIR=${OUT_DIR:-perf-results/test-loop}

OUT_DIR="$OUT_DIR$(date +%Y%m%d-%H%M%S)"
if [ ! -d $OUT_DIR ]; then
    mkdir -p $OUT_DIR
fi

if [ -n "$(ls -A $OUT_DIR)" ]; then
    echo "Warning: $OUT_DIR, backing up to /tmp"
    cp -a $OUT_DIR /tmp
    rm -r $OUT_DIR
    mkdir $OUT_DIR
fi

set -x
cargo build --target $ARCH --release

docker/cmesh build 2>&1 > perf-results/cmesh-build.log
set +x

INFOLOG=$OUT_DIR/test_info.log
echo "Test started $(date)" | tee $INFOLOG
echo "scale $SCALE, iterations $ITERATIONS" | tee -a $INFOLOG
for graph_type in complete spanning-tree la-model; do
    for i in $(seq 1 $ITERATIONS); do
        set -x
        docker/cmesh stop
        docker/cmesh rm
        set +x; echo "### Running: $graph_type iteration $i" | tee -a $INFOLOG; set -x
        docker/cmesh run $SCALE $graph_type 2>&1 | tee -a $INFOLOG
        docker/cmesh wait
        docker/cmesh cat 2>&1 > $OUT_DIR/data-$graph_type-$i.log
        set +x
    done
done
echo "Test finished $(date)" | tee -a $INFOLOG
