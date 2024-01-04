/**
 * Keep this in sync with ../common/src/types.ts
 * TODO codegen
 */

import { DocumentID } from "@dittolive/ditto";

// ----------------------------------------
// Types needed internally to this library to be a cmesh peer

/** @internal */
export type PeerId = string;

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
type SerializedPeerState = string

/** @internal */
export interface Peer {
    peer_id: PeerId;
    peer_ip_addr: string;
    peer_port: number;
    state: PeerState;
}

/** @internal */
export interface SerializedPeer {
    peer_id: PeerId;
    peer_ip_addr: string;
    peer_port: number;
    state: SerializedPeerState;   // for serialization
}


/** @internal */
export interface Heartbeat {
    sender: SerializedPeer,
    sent_at_usec: number,
}

/** @internal */
export interface HeartbeatsDoc {
    // Map from peer id to list of recent heartbeats
    beats: Map<PeerId, Heartbeat[]>;
}

/** @internal */
export interface CoordinatorInfo {
    heartbeatCollectionName: string
    heartbeatIntervalSec: number
    executionPlan: ExecutionPlan | null
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
    data: string;
}

/** @internal */
export interface PeerLog {
    log: Map<string, PeerRecord>;
}

/** @internal */
export interface PeerDoc {
    _id: DocumentID
    logs: Map<PeerId, PeerLog>
}

export class LatencyStats {
    num_events: number
    min_usec: number
    max_usec: number
    avg_usec: number
    distinct_peers: number

    constructor() {
        this.num_events = 0
        this.min_usec = 0
        this.max_usec = 0
        this.avg_usec = 0
        this.distinct_peers = 0
    }
}

export class PeerReport {
    message_latency: LatencyStats
    records_produced: number

    constructor() {
        this.message_latency = new LatencyStats()
        this.records_produced = 0
    }
}

