use std::default::Default;
use crate::types::{ExecutionPlan, CoordinatorInfo};
use crate::util;

impl Default for CoordinatorInfo {
    fn default() -> Self {
        CoordinatorInfo {
            heartbeat_collection_name: "cmesh-heartbeat".to_string(),
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
            report_collection_name: "cmesh-report".to_string(),
            peer_collection_name: "cmesh-peers".to_string(),
            min_msg_delay_msec: 100,
            max_msg_delay_msec: 1000,
            peers: Vec::new(),
        }
    }
}
