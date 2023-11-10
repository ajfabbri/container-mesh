use std::default::Default;
use crate::types::{ExecutionPlan, CoordinatorInfo};
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
        ExecutionPlan {
            start_time: util::system_time_msec() + 5000,
            test_duration_sec: 10,
            report_collection_name: REPORT_COLLECTION_NAME.to_string(),
            peer_collection_name: PEER_COLLECTION_NAME.to_string(),
            min_msg_delay_msec: 100,
            max_msg_delay_msec: 1000,
            peers: Vec::new(),
        }
    }
}