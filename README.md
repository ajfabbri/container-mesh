# Container Mesh testing for Ditto Small Peer
Created to practice working with native, containerized Ditto API dev.
(to speed up TDD for expected customer).

Idea: Test harness for p2p apps that operate in limited connectivity and/or
high-mobility environments. 

## The App
Idea is to run a bunch of containers with a peer-to-peer (p2p) ditto.live SDK
"app" which measures system metrics, and then use Linux Traffic Control (tc) (via 
[pumba](https://github.com/alexei-led/pumba)?) to simulate a degraded network,
high mobility, and other failure conditions.

### Build / Run

Tested with `DITTO_TARGET=x86_64-unknown-linux-gnu`

*Debug*
`cargo build --target $DITTO_TARGET`
`docker compose build --build-arg "FLAVOR=debug"`
`docker compose up`

*Release*
`cargo build --target $DITTO_TARGET --release`
`docker compose build --build-arg"`
`docker compose up`


### To Run in Your Environment

You'll need to create a file .secret.env that contains:

For OfflinePlayground auth (current code):

export DITTO_LICENSE="your ditto license string"

For OnlinePlayround auth:

export DITTO_APP_ID="your online playground app id"
export DITTO_PG_TOKEN="your playground token"

I'm developing on Ubuntu 22.04 for target x86_64-unknown-linux-gnu
at the moment. To build this in your environment you'll need to modify .envrc.

If you want to build on / for another platform you'll need to:

Search / replace things like:
- x86_64
- libdittoffi.so (i.e. to .dylib)
- "/lib" for target library paths, i.e. in docker/run*.sh

# Design & Notes (WIP)

### Metrics

We want to simulate peer-to-peer communication in bad networking environments,
which could be caused by:
    A. Interference and EW / Jamming
    B. High mobility nodes (frequent changes to mesh topology)
    C. Node failures.
    D. Low network bandwidth / high delay.
    .. etc.

Some metrics we'd like to collect:
- Message latency. Time between write of a value on peer_i to read of that value on peer_j.
- Resync latency. How long after losing connectivity can we catch up with current mesh state?
- Service availability. % of time we can read/write to ditto DB. (Should be near
  100%; one of our main our value props.)
- Link utilization: Would be nice to have timeseries data of the utilization of
  each active network link.
- Bandwidth savings: Versus a "resend full state on reconnect" model. Sender
  metrics would be nice, but could also calculate based on received deltas and
  their timestamps.

## Design

Start of with some simplifying assumptions:
- All containers run on same machine. For now, to make full control easier.
- All nodes (containers) have synchronized clocks. This makes event analysis
  and metrics easier.
- Timing:
    - Events will be stamped with node-local identifiers, including a timestamp.
    - Each peer will generate a write at least once a second. (to bound sync latency).

- A priori scenarios
    - Any test scenario plan can be loaded on the nodes before the test starts.

- Log collection and analysis
    - A. Use ditto.. Write logs there and analyze at end.
    - B. Write to external tracing sink.
    - C. Write to log files and collect at end of run.
  -> Try C then B?

## Work In Progress

Some TODOs:

- Write small peer test app.
    - Main node logic for test.
    - At random interval in (0.01, 0.99] seconds, write a message to peer's
      event object containing:
      - Timestamp, node_id
      - Later/nice to have: network utilization / node stats
    - Peer-to-peer bootstrapping
        - Use well-known IP and "coordinator collection" to connect to
          coordinator and get CoordinatorInfo (heartbeat details + optional
          exec. plan)

