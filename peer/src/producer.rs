use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::{self, JoinHandle};

use common::types::*;
use dittolive_ditto::prelude::*;
use log::*;
use rand::Rng;

#[derive(Clone)]
pub struct ProducerCtx {
    peer_id: PeerId,
    collection: Arc<Mutex<Collection>>,
    plan: ExecutionPlan,
    msg_index: usize,
    pub finished: Arc<AtomicBool>,
}

#[derive(PartialEq)]
pub enum ProducerStrategy {
    Update,
    #[allow(dead_code)]
    Upsert,
}

impl ProducerCtx {
    pub fn new(peer_id: PeerId, collection: Arc<Mutex<Collection>>, plan: ExecutionPlan) -> Self {
        Self {
            peer_id,
            collection,
            plan,
            msg_index: 0,
            finished: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub fn producer_send(prod_ctx: &mut ProducerCtx, strategy: ProducerStrategy) {
    let hbc_lock = prod_ctx.collection.lock().unwrap();
    let rec = PeerRecord::default();
    // TODO fill in rec.data to pad size as desired
    let rec_path = format!(
        "logs['{}']['{}']",
        prod_ctx.peer_id.to_string().as_str(),
        prod_ctx.msg_index.to_string().as_str()
    );
    if strategy == ProducerStrategy::Upsert {
        debug!("---> producer_send: upsert path: {} -> {:?}", rec_path, rec);
        let res = hbc_lock.find_by_id(&prod_ctx.plan.peer_doc_id).exec();
        match res {
            Ok(doc) => {
                let r = doc.typed::<PeerDoc>();
                match r {
                    Ok(mut pdoc) => {
                        debug!("---> producer upsert: find_by_id: {:?}", pdoc);
                        let my_log = pdoc.logs.get_mut(&prod_ctx.peer_id).unwrap();
                        my_log.insert(prod_ctx.msg_index.to_string(), rec);
                        hbc_lock.upsert(pdoc).unwrap();
                    }
                    Err(e) => {
                        error!("--> upsert PeerDoc deser Error {:?}", e);
                        info!("received peer doc {:?}", doc);
                    }
                }
            }
            Err(e) => {
                error!("---> producer_send: find_by_id error: {:?}", e);
            }
        }

    } else {
        debug!("---> producer_send: update path: {} -> {:?}", rec_path, rec);
        hbc_lock
            .find_by_id(&prod_ctx.plan.peer_doc_id)
            .update(|mut_doc| {
                debug!("---> producer set {} to {:?}", rec_path, rec);
                let mut_doc = mut_doc.unwrap();
                mut_doc
                    .set(rec_path.as_str(), rec.clone())
                    .expect("producer mutate doc");
            })
            .expect("producer mutate doc");
    }
    prod_ctx.msg_index += 1;
}

pub fn producer_start(prod_ctx: ProducerCtx) -> JoinHandle<Result<u64, std::io::Error>> {
    info!("--> producer_start");
    let t = thread::spawn(move || -> Result<u64, std::io::Error> { producer_loop(prod_ctx) });
    t
}

pub fn producer_stop(prod_ctx: &ProducerCtx) {
    info!("--> producer_stop");
    prod_ctx
        .finished
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

// producer timer loop
pub fn producer_loop(mut prod_ctx: ProducerCtx) -> Result<u64, std::io::Error> {
    // TODO timing / message rate, etc.
    let mut count = 0;
    while !prod_ctx.finished.load(std::sync::atomic::Ordering::Relaxed) {
        producer_send(&mut prod_ctx, ProducerStrategy::Update);
        count += 1;
        let mut rng = rand::thread_rng();
        let msec = rng.gen_range(prod_ctx.plan.min_msg_delay_msec..
            prod_ctx.plan.max_msg_delay_msec);
        std::thread::sleep(std::time::Duration::from_millis(msec as u64));
    }
    Ok(count)
}
