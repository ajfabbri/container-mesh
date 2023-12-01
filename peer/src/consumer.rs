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
    util::{print_cdoc, system_time_usec},
};
use dittolive_ditto::prelude::*;

pub struct PeerConsumer {
    local_id: PeerId,
    next_record_by_peer: HashMap<PeerId, usize>,
    msg_latency: LatencyStats,
    msg_latency_total: u64,
    // To keep subscription alive as needed
    #[allow(dead_code)]
    subscription: Subscription,
    pub live_query: Option<LiveQuery>,
}

impl PeerConsumer {
    fn new(local_id: PeerId, subscription: Subscription) -> Self {
        Self {
            local_id,
            next_record_by_peer: HashMap::new(),
            msg_latency: LatencyStats::new(),
            msg_latency_total: 0,
            subscription,
            live_query: None,
        }
    }

    fn peek_next_idx(&self, peer_id: &PeerId) -> usize {
        let i = self.next_record_by_peer.get(peer_id);
        i.unwrap_or(&0).clone()
    }

    fn set_next_idx(&mut self, peer_id: PeerId, i: usize) {
        self.next_record_by_peer.insert(peer_id, i);
    }

    fn process_peer(&mut self, id: PeerId, log: &HashMap<String, PeerRecord>) {
        let now = system_time_usec();
        let mut i = self.peek_next_idx(&id);
        debug!("--> process_peer {} w/ log len {}", id, log.len());
        loop {
            let rec = log.get(i.to_string().as_str());
            if rec.is_none() {
                break;
            }
            let r = rec.unwrap();
            let latency = now - r.timestamp;
            self.msg_latency_total += latency;
            self.msg_latency.num_events += 1;
            self.msg_latency.min_usec = cmp::min(self.msg_latency.min_usec, latency);
            self.msg_latency.max_usec = cmp::max(self.msg_latency.max_usec, latency);
            self.msg_latency.avg_usec = self.msg_latency_total / self.msg_latency.num_events;
            debug!("--> got peer record {:?} w/ latency {}", r, latency);
            i += 1;
        }
        self.set_next_idx(id, i);
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
        stats.distinct_peers = self.next_record_by_peer.len();
        stats
    }
}

pub fn consumer_create_collection(pctx: &PeerContext) -> Result<Collection, Box<dyn Error>> {
    let store = pctx.ditto.store();
    let plan = pctx.get_plan().unwrap();
    let cc = store.collection(&plan.peer_collection_name)?;
    let mylog: HashMap<String, PeerRecord> = HashMap::new();
    let mut peer_logs = HashMap::new();
    peer_logs.insert(pctx.id.clone(), mylog);
    let mut logs = HashMap::new();
    logs.insert(pctx.id.clone(), HashMap::<String, PeerRecord>::new());

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
