# Typescript Cmesh Peer Library

*For the ditto.live SDK scale testing system [container mesh
(cmesh)](../../README.md).*

Allows typescript apps to participate as a peer in a the cmesh test execution
environment.

We use this to develop peer-to-peer (p2p) test apps which use the cmesh
coordinator to: 

- Add themselves to the test as executors (peers).
- Discover all nodes participating and estabilish a peer-to-peer mesh with
  them.
- Receive a plan (parameters) for the test and coordinate parallel execution.
- Report measured metrics.

The original cmesh peer and coordinator apps are written in Rust. This
typescript library allows typescript programs to also particpate with the rust
coordinator and even rust peers (i.e. a p2p mesh test with Rust SDK peers and
TS/JS SDK peers.

## TODOs
- [ ] Style cleanup. *sad face* use camelCase, etc.
- [x] Mesh connection per coordinator's graph
- [x] Finish producer / consumer and test against rust peers.
- [ ] Use codegen to generate structs / objects for rust and typescript.
