use log::*;
use std::{
    cmp,
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use crate::PeerContext;
use common::{
    types::*,
    util::{print_cdoc, system_time_msec}, default::PEER_LOG_SIZE,
};
use dittolive_ditto::prelude::*;

pub struct PeerConsumer {
    local_id: PeerId,
    last_ts_idx_by_peer: HashMap<PeerId, (u64, u32)>,
    msg_latency: LatencyStats,
    msg_latency_total: u64,
    // To keep subscription alive as needed
    #[allow(dead_code)]
    subscription: Subscription,
    pub live_query: Option<LiveQuery>,
}

fn incr_wrap(i: u32, max: u32) -> u32 {
    let mut r = i + 1;
    if r > max {
        r = 0;
    }
    r
}

impl PeerConsumer {
    fn new(local_id: PeerId, subscription: Subscription) -> Self {
        Self {
            local_id,
            last_ts_idx_by_peer: HashMap::new(),
            msg_latency: LatencyStats::new(),
            msg_latency_total: 0,
            subscription,
            live_query: None,
        }
    }

    // get timestamp of last record consumed, and expected next index
    fn get_ts_idx(&self, peer_id: &PeerId) -> (u64, u32) {
        let r = self.last_ts_idx_by_peer.get(peer_id);
        r.unwrap_or(&(0,0)).clone()
    }

    // set last timestamp and log index we consumed for this peer
    fn set_consumed_ts_idx(&mut self, peer_id: PeerId, ts: u64, mut i: u32, max_i: u32) {
        i = incr_wrap(i, max_i);
        self.last_ts_idx_by_peer.insert(peer_id, (ts, i));
    }

    fn process_peer(&mut self, id: PeerId, pl: &PeerLog) {
        let now = system_time_msec();
        let (mut ts, mut i) = self.get_ts_idx(&id);
        debug!("--> process_peer {} w/ log len {}", id, pl.log.len());
        loop {
            let rec = pl.log.get(i.to_string().as_str());
            if rec.is_none() || rec.unwrap().timestamp < ts {
                // we either got no record, or have wrapped to an old one
                break;
            }
            let r = rec.unwrap();
            let latency = now - r.timestamp;
            self.msg_latency_total += latency;
            self.msg_latency.num_events += 1;
            self.msg_latency.min_msec = cmp::min(self.msg_latency.min_msec, latency);
            self.msg_latency.max_msec = cmp::max(self.msg_latency.max_msec, latency);
            self.msg_latency.avg_msec = self.msg_latency_total / self.msg_latency.num_events;
            debug!("--> got peer record {:?} w/ latency {}", r, latency);
            i = incr_wrap(i, PEER_LOG_SIZE-1);
            ts = r.timestamp
        }
        self.set_consumed_ts_idx(id, ts, i, PEER_LOG_SIZE-1);
    }

    fn process_peer_doc(&mut self, pdoc: &PeerDoc) {
        for (peer_id, log) in &pdoc.logs {
            if peer_id == &self.local_id {
                // don't process your own records
                continue;
            }
            self.process_peer(peer_id.to_string(), log);
        }
    }

    pub fn get_message_latency(&self) -> LatencyStats {
        let mut stats = self.msg_latency.clone();
        stats.distinct_peers = self.last_ts_idx_by_peer.len();
        stats
    }
}

pub fn consumer_create_collection(pctx: &PeerContext) -> Result<Collection, Box<dyn Error>> {
    let store = pctx.ditto.store();
    let plan = pctx.get_plan().unwrap();
    let cc = store.collection(&plan.peer_collection_name)?;
    let mut logs = HashMap::new();
    logs.insert(pctx.id.clone(), PeerLog::new());

    let doc = PeerDoc {
        _id: plan.peer_doc_id.clone(),
        logs,
    };
    cc.upsert(doc)?;
    Ok(cc)
}

pub type PeerConsumerRef = Arc<Mutex<PeerConsumer>>;

pub fn consumer_start(pctx: &PeerContext) -> Result<PeerConsumerRef, Box<dyn Error>> {
    let coll = pctx.peer_collection.as_ref().unwrap().lock();
    let plan = pctx.get_plan().unwrap();
    let peer_doc_id = plan.peer_doc_id.clone();
    let query = coll.as_ref().unwrap().find_by_id(&peer_doc_id);
    info!(
        "--> consumer_start for coll {} w/ doc id {}",
        plan.peer_collection_name,
        peer_doc_id.to_query_compatible(StringPrimitiveFormat::WithoutQuotes)
    );

    let _consumer = Arc::new(Mutex::new(PeerConsumer::new(pctx.id.clone(), query.subscribe())));
    let consumer = _consumer.clone();
    let live_query = query
        .observe_local(move |doc: Option<BoxedDocument>, event| {
            trace!("-> observe peer event {:?}", event);
            if doc.is_none() {
                return;
            }
            let r = doc.as_ref().unwrap().typed::<PeerDoc>();
            match r {
                Ok(pdoc) => {
                    //let p = doc.unwrap().to_cbor().unwrap();
                    //print_cdoc(&p).unwrap();
                    consumer.lock().unwrap().process_peer_doc(&pdoc);
                }
                Err(e) => {
                    error!("PeerDoc deser Error {:?}", e);
                    let p = doc.unwrap().to_cbor().unwrap();
                    info!("received peer doc:");
                    print_cdoc(&p).unwrap();
                }
            }
        })
        .unwrap();
    _consumer.lock().unwrap().live_query = Some(live_query);
    Ok(_consumer)
}
