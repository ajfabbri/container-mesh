#!/usr/bin/env bash
set -eu -o pipefail -x
cd "$(dirname "${BASH_SOURCE[0]}")/.."
source .env

export ARCH=${ARCH:-x86_64-unknown-linux-gnu}
export FLAVOR=release
export RUST_LOG=warning

# Automated iterations of container mesh tests.

SCALE=${SCALE:-20}
ITERATIONS=${ITERATIONS:-4}
OUT_DIR=${OUT_DIR:-perf-results/test-loop}
# complete connection graphs don't scale, stop trying past this number of peers
COMPLETE_MAX_SCALE=${COMPLETE_MAX_SCALE:-100}

OUT_DIR="$OUT_DIR-$SCALE-$(date +%Y%m%d-%H%M%S)"
if [ ! -d $OUT_DIR ]; then
    mkdir -p $OUT_DIR
fi

if [ -n "$(ls -A $OUT_DIR)" ]; then
    echo "Warning: $OUT_DIR, backing up to /tmp"
    cp -a $OUT_DIR /tmp
    rm -r $OUT_DIR
    mkdir $OUT_DIR
fi

if [ $FLAVOR = "release" ]
then
    set -x
    cargo build --target $ARCH --release
    set +x
else
    set -x
    cargo build --target $ARCH
    set +x
fi

docker/cmesh build 2>&1 > perf-results/cmesh-build.log
set +x

INFOLOG=$OUT_DIR/test_info.log
echo "Test started $(date)" | tee $INFOLOG
echo "scale $SCALE, iterations $ITERATIONS" | tee -a $INFOLOG
for graph_type in "complete" "spanning-tree" "la-model"; do
    if [ $graph_type = "complete" ] && [ $SCALE -gt $COMPLETE_MAX_SCALE ]; then
        echo "*** Skipping complete graph at scale $SCALE ***"
        continue
    fi
    for i in $(seq 1 $ITERATIONS); do
        set -x
        docker/cmesh stop
        docker/cmesh rm
        set +x; echo "### Running: $graph_type iteration $i" | tee -a $INFOLOG; set -x
        CONN_GRAPH_TYPE=$graph_type docker/cmesh run $SCALE 2>&1 | tee -a $INFOLOG
        docker/cmesh wait
        docker/cmesh cat 2>&1 > $OUT_DIR/data-$graph_type-$i.log
        docker/cmesh cp-coord $OUT_DIR
        set +x
    done
done
echo "Test finished $(date)" | tee -a $INFOLOG
