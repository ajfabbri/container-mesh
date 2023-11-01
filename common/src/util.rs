use std::time;

pub fn system_time_msec() -> u64 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("SystemTime::now")
        .as_millis().try_into().unwrap()
}
