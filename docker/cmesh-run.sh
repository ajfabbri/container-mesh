#!/usr/bin/env bash
set -euo pipefail

# XXX work in progress
set -x
cd "$(dirname "${BASH_SOURCE[0]}")/.."
set +x

# config
ARCH=${ARCH:-x86_64-unknown-linux-gnu}

usage() {
    echo "Usage: $0 <scale-factor>  [debug]"
    echo "  note: set ARCH env var to override default $ARCH"
    exit 1
}

build_coord() {
    # take all args
    local build_args="$1"

    # don't log env vars
    set -x
    pre='docker build '
    post=' -t cmesh-coord -f docker/Dockerfile.coord .'
    echo "$pre$post"
    # XXX set +x
    eval "$pre" "$build_args" "$post"
    set -x
}

build_peer() {
    local build_args="$1"
    # build and run peer container
    eval 'docker build' "$build_args" '-t cmesh-peer -f docker/Dockerfile.peer .'
}

run_coord() {
    docker run -d --rm --name cmesh-coord cmesh-coord
}

stop_coord() {
    # stop all containers with cmesh-coord tag
    cs=$(docker ps -q -f ancestor=cmesh-coord)
    if [[ -n $cs ]]; then
        docker stop $cs
    fi
}


run_peers() {
    local s=$1
    local scale=$((s-1))
    for (( i = 0; i<scale; i++)); do
        set -x
        docker run -d --rm --name cmesh-peer-$i cmesh-peer
        set +x
    done
}

stop_peers() {
    # stop all containers with cmesh-peer tag
    cs=$(docker ps -q -f ancestor=cmesh-peer)
    if [[ -n $cs ]]; then
        docker stop $cs
    fi
}


scale=$1
flavor=${2:-release}

if [[ ! $scale =~ ^-?[0-9]+$ ]]; then
    usage
fi

# main
set -x
build_args=""

if [[ $flavor = "debug" ]]; then
    build_args+=" --build-arg FLAVOR=debug"
fi

set +x
pwd
source .secret.env
#build_args+=" --build-arg ARCH=$ARCH"
#build_args+=" --build-arg DITTO_APP_ID=$DITTO_APP_ID"
#build_args+=" --build-arg DITTO_PG_TOKEN=$DITTO_PG_TOKEN"
#build_args+=" --build-arg DITTO_LICENSE=$DITTO_LICENSE"
set -x
#echo $build_args
#stop_coord
#stop_peers
#build_peer "$build_args"
#build_coord "$build_args"
run_coord
run_peers $scale
