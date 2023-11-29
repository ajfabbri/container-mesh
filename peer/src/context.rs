use log::info;
use std::{
    error::Error,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use common::{types::*, util::system_time_msec};
use dittolive_ditto::prelude::*;

use crate::HeartbeatCtx;
use crate::consumer::PeerConsumer;

pub struct PeerContext {
    pub id: PeerId,
    pub ditto: Ditto,
    pub coord_addr: Option<String>,
    pub coord_doc_id: Option<DocumentId>,
    pub coord_info: Option<CoordinatorInfo>,
    // Keep a copy of our last transport config so we can modify and re-set it.
    pub transport_config: Option<TransportConfig>,
    #[allow(dead_code)]
    pub hb_doc_id: Option<DocumentId>,
    pub hb_ctx: Option<HeartbeatCtx>,
    pub hb_thread: Option<JoinHandle<Result<(), std::io::Error>>>,
    #[allow(dead_code)]
    pub start_time_msec: u64,
    pub local_ip: String,
    pub state: Arc<Mutex<PeerState>>,
    pub peer_collection: Option<Arc<Mutex<Collection>>>,
    pub peer_consumer: Option<PeerConsumer>,
}

impl PeerContext {
    pub fn new(device_name: &str, ditto: Ditto, local_ip: &str) -> Self {
        Self {
            id: random_peer_id(Some(&device_name)),
            ditto,
            coord_addr: None,
            coord_doc_id: None,
            coord_info: None,
            transport_config: None,
            hb_doc_id: None,
            hb_ctx: None,
            hb_thread: None,
            start_time_msec: system_time_msec(),
            local_ip: local_ip.to_string(),
            state: Arc::new(Mutex::new(PeerState::Init)),
            peer_collection: None,
            peer_consumer: None,
        }
    }

    pub fn get_plan(&self) -> Option<ExecutionPlan> {
        let ci = self.coord_info.as_ref()?;
        ci.execution_plan.clone()
    }

    pub fn state_transition(
        &mut self,
        existing: Option<PeerState>,
        new: PeerState,
    ) -> Result<(), Box<dyn Error>> {
        let mut state = self.state.lock().unwrap();
        assert!(existing.is_none() || existing.unwrap() == *state);
        info!("--> state_transition: {:?} -> {:?}", state, new);
        *state = new;
        Ok(())
    }
}
