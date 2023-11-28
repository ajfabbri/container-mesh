#!/usr/bin/env bash
set -euo pipefail

# XXX work in progress
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# config
ARCH=${ARCH:-x86_64-unknown-linux-gnu}
FLAVOR=${FLAVOR:-release}

COORD_PORT=4001
COORD_ADDR="10.1.0.2"
PEER_BEGIN_PORT=5100
IP_RANGE="10.1.1.0/24"
SUBNET="10.1.0.0/16"
GATEWAY="10.1.0.1"

usage() {
    echo "Usage: $0 build | stop | run <scale-factor> | clean | watch"
    echo "  ARCH=$ARCH      FLAVOR=$FLAVOR"
    exit 1
}

build_coord() {
    # take all args
    local build_args="$1"

    pre='docker build '
    post=' -t cmesh-coord -f docker/Dockerfile.coord .'
    eval "$pre" "$build_args" "$post"
}

build_peer() {
    local build_args="$1"
    # build and run peer container
    eval 'docker build' "$build_args" '-t cmesh-peer -f docker/Dockerfile.peer .'
}

run_coord() {
    local scale=$1
    docker run -d --rm --name cmesh-coord --label cmesh \
      --network=mesh --ip=$COORD_ADDR \
    cmesh-coord --bind-addr $COORD_ADDR --bind-port $COORD_PORT --min-peers $scale
}

stop_coord() {
    cs=$(docker ps -q -f ancestor=cmesh-coord)
    if [[ -n $cs ]]; then
        docker stop $cs
    fi
}


run_peers() {
    local scale=$1
    for (( i = 0; i<scale; i++)); do
        block_sz=10
        beginport=$((5100 + ( i * block_sz) ))
        endport=$((beginport + block_sz - 1))
        set -x
        docker run -d --rm --name cmesh-peer-$i --label cmesh \
          --network=mesh --expose "$beginport-$endport" \
        cmesh-peer --coord-addr $COORD_ADDR --coord-port $COORD_PORT \
          --bind-port $beginport --device-name peer$i
        set +x
    done
}

stop_peers() {
    cs=$(docker ps -q -f ancestor=cmesh-peer)
    if [[ -n $cs ]]; then
        docker stop $cs
    fi
}

do_run() {
    local scale=$1
    # create bridge network "mesh"
    docker network inspect mesh >/dev/null 2>&1 || \
    docker network create -d bridge \
      --ip-range $IP_RANGE --subnet $SUBNET --gateway $GATEWAY \
      mesh
    run_coord $((scale - 1))    # allow one peer to fail?
    run_peers $scale
}

do_stop() {
    stop_peers || true
    stop_coord || true
    docker network rm mesh
}

do_build() {
    local arch=$1
    local flavor=$2
    # main
    build_args="--build-arg ARCH=$arch"

    if [[ $flavor = "debug" ]]; then
        build_args+=" --build-arg FLAVOR=debug"
    fi
    echo "--> build coordinator"
    build_coord "$build_args"
    echo "--> build peer"
    build_peer "$build_args"
}

do_clean() {
    do_stop
    # clean up docker disk space
    docker system prune -f
}

do_watch() {
    cids=$(docker ps -q -f label=cmesh)
    if [[ -z $cids ]]; then
        echo "No cmesh containers running"
        exit 1
    fi
    cleanup()
    {
       kill "${pids[@]}"
    }

    trap cleanup EXIT

    for cid in ${cids[@]}; do
        echo "--> watching $cid"
        (docker logs -f -t --tail=10 "$cid" | sed -e "s/^/$cid: /")&
        pids+=($!)
        echo pids: ${pids[@]}
    done
    wait
}

# === main ===

# parse cli args
POSITIONAL_ARGS=()

while [[ $# -gt 0 ]]; do
  case $1 in
    run)
      MODE=run
      if [ -z ${2+x} ] || [[ ! $2 =~ ^-?[0-9]+$ ]]; then
            echo "** Missing scale parameter **"
            usage
      fi
      SCALE=$2
      shift; shift
      ;;
    build)
      MODE=build
      shift # past value
      ;;
    stop)
      MODE=stop
      shift # past argument
      ;;
    clean)
      MODE=clean
      shift # past argument
      ;;
    watch)
      MODE=watch
      shift # past argument
      ;;
    -*|--*)
      echo "Unknown option $1"
      exit 1
      ;;
    *)
      POSITIONAL_ARGS+=("$1") # save positional arg
      shift # past argument
      ;;
  esac
done

set -- "${POSITIONAL_ARGS[@]}" # restore positional parameters

if [ -z ${MODE+x} ] ; then usage; fi

if [[ $MODE == "run" ]]; then
    do_run $SCALE
elif [[ $MODE == "stop" ]]; then
    do_stop
elif [[ $MODE == "build" ]]; then
    do_build $ARCH $FLAVOR
elif [[ $MODE == "clean" ]]; then
    do_clean
elif [[ $MODE == "watch" ]]; then
    do_watch
fi
