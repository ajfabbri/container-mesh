use std::default::Default;
use dittolive_ditto::prelude::DocumentId;

use crate::types::{ExecutionPlan, CoordinatorInfo, PeerRecord};
use crate::util;

pub const HEARTBEAT_COLLECTION_NAME: &str = "cmesh-heartbeat";
pub const REPORT_COLLECTION_NAME: &str = "cmesh-report";
pub const PEER_COLLECTION_NAME: &str = "cmesh-peers";

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
            min_msg_delay_msec: 10,
            max_msg_delay_msec: 500,
            peers: Vec::new(),
        }
    }
}

impl Default for PeerRecord {
    fn default() -> Self {
        PeerRecord {
            timestamp: util::system_time_usec(),
            data: String::new(),
        }
    }
}
