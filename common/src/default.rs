use std::default::Default;
use dittolive_ditto::prelude::DocumentId;

use crate::types::*;
use crate::util;

pub const COORD_COLLECTION_NAME: &str = "cmesh-coord";
pub const HEARTBEAT_COLLECTION_NAME: &str = "cmesh-heartbeat";
pub const REPORT_COLLECTION_NAME: &str = "cmesh-report";
pub const PEER_COLLECTION_NAME: &str = "cmesh-peers";
pub const PEER_LOG_SIZE: u32 = 16;
pub const GRAPH_SPANNING_MAX_DEGREE: usize = 3;
pub const GRAPH_LA_CLIQUE_SIZE: usize = 4;
pub const QUERY_POLL_SEC: u64 = 2;  // peer delay between polling for coord. info
pub const REPORT_PROPAGATION_SEC: u64 = 2;  // peer wait before shutting down
pub const HEARTBEAT_SEC: u64 = 2; // peer delay between heartbeat writes


impl Default for CoordinatorInfo {
    fn default() -> Self {
        CoordinatorInfo {
            heartbeat_collection_name: HEARTBEAT_COLLECTION_NAME.to_string(),
            heartbeat_interval_sec: 5,
            execution_plan: None,
        }
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        let some_id = format!("{:x}", rand::random::<u64>());
        ExecutionPlan {
            // XXX TODO base start time on peers being ready
            start_time: util::system_time_msec() + 10000,
            test_duration_sec: 60,
            report_collection_name: REPORT_COLLECTION_NAME.to_string(),
            peer_collection_name: PEER_COLLECTION_NAME.to_string(),
            peer_doc_id: DocumentId::from(some_id.as_bytes()),
            min_msg_delay_msec: 900,
            max_msg_delay_msec: 1100,
            peers: Vec::new(),
            connections: PeerGraph::new(),
        }
    }
}

impl Default for PeerRecord {
    fn default() -> Self {
        PeerRecord {
            timestamp: util::system_time_msec(),
            data: String::new(),
        }
    }
}
