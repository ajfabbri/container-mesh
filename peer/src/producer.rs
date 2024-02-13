use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::{self, JoinHandle};

use common::default::*;
use common::types::*;
use dittolive_ditto::prelude::*;
use log::*;
use rand::Rng;

#[derive(Clone)]
pub struct ProducerCtx {
    peer_id: PeerId,
    collection: Arc<Mutex<Collection>>,
    plan: ExecutionPlan,
    msg_index: i32,
    pub finished: Arc<AtomicBool>,
}

impl ProducerCtx {
    pub fn new(peer_id: PeerId, collection: Arc<Mutex<Collection>>, plan: ExecutionPlan) -> Self {
        Self {
            peer_id,
            collection,
            plan,
            msg_index: -1,
            finished: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ProducerCtx {
    fn get_next_index(&mut self) -> u32 {
        self.msg_index += 1;
        if self.msg_index >= PEER_LOG_SIZE as i32 {
            self.msg_index = 0;
        }
        self.msg_index.try_into().unwrap()
    }
}

pub fn producer_send(prod_ctx: &mut ProducerCtx) {
    // get next index for circulare peer log
    let next_index = prod_ctx.get_next_index();
    let hbc_lock = prod_ctx.collection.lock().unwrap();
    let rec = PeerRecord::default();
    // TODO fill in rec.data to pad size as desired
    let rec_path = format!(
        "logs['{}']['log']['{}']",
        &prod_ctx.peer_id,
        next_index.to_string().as_str()
    );
    debug!("---> producer_send: update path: {} -> {:?}", rec_path, rec);
    let id_op = hbc_lock.find_by_id(&prod_ctx.plan.peer_doc_id);
    id_op
        .update(|mut_doc| {
            debug!("---> producer set {} to {:?}", rec_path, rec);
            let mut_doc = mut_doc.unwrap();
            mut_doc
                .set(rec_path.as_str(), rec.clone())
                .expect("producer mutate doc rec");
        })
        .expect("producer mutate rec");
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
    let mut count: u64 = 0;
    while !prod_ctx.finished.load(std::sync::atomic::Ordering::Relaxed) {
        producer_send(&mut prod_ctx);
        count += 1;
        let mut rng = rand::thread_rng();
        let msec =
            rng.gen_range(prod_ctx.plan.min_msg_delay_msec..prod_ctx.plan.max_msg_delay_msec);
        std::thread::sleep(std::time::Duration::from_millis(msec as u64));
    }
    Ok(count)
}
