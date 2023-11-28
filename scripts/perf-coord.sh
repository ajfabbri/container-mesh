#!/usr/bin/env bash
set -eu -o pipefail -x
cd "$(dirname "${BASH_SOURCE[0]}")"
source ../.env

ARCH=${ARCH:-x86_64-unknown-linux-gnu}
FLAVOR=${FLAVOR:-"debug"}

if [ ! -d ../perf-output ]; then
    mkdir ../perf-output
fi
cd ../perf-output

if [ ! -f flamegraph.pl ]; then
    wget https://raw.githubusercontent.com/brendangregg/FlameGraph/master/flamegraph.pl
    chmod +x flamegraph.pl
fi

# The struggle of running perf on a container process..

# tried getting container root path for perf record --symfs...
#cfs_path=$(docker inspect cmesh-coord | jq '.[0].GraphDriver.Data.MergedDir')/merged
#cfs_bin=$cfs_path/root/$ARCH/$FLAVOR/
#cfs_path=/coord-root

# my kernel lacked CONFIG_CGROUP_PERF
#sudo perf record -p $cpid -F 50 --call-graph=dwarf -- sleep 2
#sudo perf record -F 49 --call-graph=dwarf -a -G $cgroup_id -- sleep 2

# Use nsenter to run perf commands..

# host pid
hpid=$(pgrep cmesh-coord)
# container pid
cpid=$(grep NSpid /proc/$hpid/status | awk '{print $3}')

sudo nsenter -t $hpid -n -m -u -i -p -- \
    perf record -F 60 -p $cpid \
    --call-graph dwarf \
    -- sleep 2

sudo nsenter -t $hpid -n -m -u -i -p -- \
  perf report -n --stdio --no-children -g folded,0,caller,count -s comm | \
    awk '/^ / { comm = $3 } /^[0-9]/ { print comm ";" $2, $1 }' \
    > out.perf-coord
# ^ awk-foo to get collapsed stacks in flamegraph's desired format

# generate flamegraph
./flamegraph.pl out.perf-coord > perf-coord.svg

