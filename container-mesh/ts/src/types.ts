/**
 * Keep this in sync with ../common/src/types.ts
 * TODO codegen
 */

import { DocumentID } from "@dittolive/ditto";

// Types needed internally to this library to be a cmesh peer
/** @internal */
export type PeerId = String;

/** @internal */
export interface PeerGraph {
    nmap: Map<string, Set<string>>;
}

/** @internal */
export enum PeerState {
    Init,       // Alive, reporting to coord.
    Ready,      // Have test plan, ready to execute
    Running,    // Executing
    Reporting,  // Finished test, outputting results
    Shutdown,   // Done, exiting
}

/** @internal */
export interface Peer {
    peer_id: PeerId;
    peer_ip_addr: string;
    peer_port: number;
    peer_state: PeerState;
}

/** @internal */
export interface Heartbeat {
    sender: Peer,
    sent_at_usec: number,
}

/** @internal */
export interface HeartbeatsDoc {
    // Map from peer id to list of recent heartbeats
    beats: Map<PeerId, Heartbeat[]>;
}

/** @internal */
export interface CoordinatorInfo {
    heartbeat_collection_name: string;
    heartbeat_interval_sec: number;
    executionPlan?: ExecutionPlan;
}

/** @internal */
export interface ExecutionPlan {
    start_time: number;
    test_duration_sec: number;
    report_collection_name: string;
    peer_collection_name: string;
}

/** @internal */
export interface PeerRecord {
    timestamp: number;
    data: String;
}

/** @internal */
export interface PeerLog {
    log: Map<string, PeerRecord>;
}

/** @internal */
export interface PeerDoc {
    _id: DocumentID;
    logs: Map<PeerId, PeerLog>
}

