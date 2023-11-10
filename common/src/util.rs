use std::time;
use crate::types::Heartbeat;

pub fn system_time_msec() -> u64 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("SystemTime::now")
        .as_millis().try_into().unwrap()
}

impl Heartbeat {
    pub fn update_timestamp(&mut self) {
        self.sent_at_msec = system_time_msec();
    }
}
