use std::{error::Error, collections::HashMap};

use common::{types::{PeerRecord, PeerDoc}, util::print_cdoc};
use dittolive_ditto::prelude::*;
use crate::PeerContext;


struct PeerConsumer {
    // TODO stats
    event_count: usize,
    // To keep subscription alive as needed
    #[allow(dead_code)]
    subscription: Subscription,
}

pub fn consumer_create_collection(pctx: &PeerContext) -> Result<Collection, Box<dyn Error>> {
    let store = pctx.ditto.store();
    let plan = pctx
        .coord_info
        .as_ref()
        .unwrap()
        .execution_plan
        .as_ref()
        .unwrap();
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

impl PeerConsumer {
    fn process_peer(&mut self, _pdoc: PeerDoc) {
        println!("--> process_peer {:?}", _pdoc);
        self.event_count += 1;
    }
}

pub fn consumer_start(pctx: &PeerContext) -> Result<LiveQuery, Box<dyn Error>> {
    let coll = pctx.peer_collection.as_ref().unwrap().lock();
    let plan = pctx
        .coord_info
        .as_ref()
        .unwrap()
        .execution_plan
        .as_ref()
        .unwrap();
    let peer_doc_id = plan.peer_doc_id.clone();
    let query = coll.as_ref().unwrap().find_by_id(peer_doc_id.clone());
    println!(
        "--> XXX consumer_start for coll {} w/ doc id {}",
        plan.peer_collection_name, peer_doc_id
    );

    let mut consumer = PeerConsumer {
        event_count: 0,
        subscription: query.subscribe(),
    };
    let live_query = query
        .observe_local(move |doc: Option<BoxedDocument>, event| {
            println!("XXX -> observe peer event {:?}", event);
            if doc.is_none() {
                return;
            }
            let r = doc.as_ref().unwrap().typed::<PeerDoc>();
            match r {
                Ok(pdoc) => {
                    //let p = doc.unwrap().to_cbor().unwrap();
                    //print_cdoc(&p).unwrap();
                    consumer.process_peer(pdoc);
                }
                Err(e) => {
                    println!("PeerDoc deser Error {:?}", e);
                    let p = doc.unwrap().to_cbor().unwrap();
                    println!("received peer doc:");
                    print_cdoc(&p).unwrap();
                }
            }
        })
        .unwrap();
    Ok(live_query)
}
