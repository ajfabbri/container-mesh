/**
 * Keep this in sync with ../common/src/types.ts
 * Note: we use snake_case field names for serialized fields to match the
 * actual document paths/keys.
 * TODO codegen
 */

import { DocumentID } from "@dittolive/ditto";

// ----------------------------------------
// Types needed internally to this library to be a cmesh peer

/** @internal */
export type PeerId = string;

// map from peer id to set of peer ids it should connect to
/** @internal */
export interface PeerGraph {
    [key: string]: Array<string>
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
    sent_at_msec: number,
}

/** @internal */
export interface HeartbeatsDoc {
    // Map from peer id to list of recent heartbeats
    beats: Map<PeerId, Heartbeat[]>;
}

/** @internal */
export interface CoordinatorInfo {
    heartbeat_collection_name: string
    heartbeat_interval_sec: number
    execution_plan: ExecutionPlan | null
}

/** @internal */
export interface ExecutionPlan {
    start_time: number
    test_duration_sec: number
    report_collection_name: string
    peer_collection_name: string
    peer_doc_id: DocumentID
    min_msg_delay_msec: number
    max_msg_delay_msec: number
    peers: Peer[]
    connections: PeerGraph
}

/** @internal */
export class PeerRecord {
    timestamp: number;
    data: string;
    constructor() {
        this.timestamp = Date.now()
        this.data = ""
    }
}

/** @internal */
export interface PeerLog {
    log: { [key: string]: PeerRecord }
}

export type PeerLogs = { [key: PeerId]: PeerLog }

/** @internal */
export interface PeerDoc {
    _id: DocumentID
    logs: PeerLogs
}

export class LatencyStats {
    num_events: number
    min_msec: number
    max_msec: number
    avg_msec: number
    distinct_peers: number

    constructor() {
        this.num_events = 0
        this.min_msec = Number.MAX_SAFE_INTEGER
        this.max_msec = 0
        this.avg_msec = 0
        this.distinct_peers = 0
    }
}

export class PeerReport {
    message_latency: LatencyStats
    records_produced: number

    constructor(latency: LatencyStats, records: number) {
        this.message_latency = latency
        this.records_produced = records
    }
}

