use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::{self, JoinHandle};

use common::types::*;
use dittolive_ditto::prelude::*;

#[derive(Clone)]
pub struct ProducerCtx {
    peer_id: PeerId,
    collection: Arc<Mutex<Collection>>,
    plan: ExecutionPlan,
    msg_index: usize,
    pub finished: Arc<AtomicBool>,
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

pub fn producer_send(prod_ctx: &mut ProducerCtx) {
    let hbc_lock = prod_ctx.collection.lock().unwrap();
    let rec = PeerRecord::default();
    // TODO fill in rec.data to pad size as desired

    //let rec_path = &vec![
    //    "logs",
    //    prod_ctx.peer_id.to_string().as_str(),
    //    prod_ctx.msg_index.to_string().as_str(),
    //]
    //.join(".");
    let rec_path = format!(
        "logs['{}']['{}']",
        prod_ctx.peer_id.to_string().as_str(),
        prod_ctx.msg_index.to_string().as_str()
    );
    println!("---> producer_send: update path: {} -> {:?}", rec_path, rec);
    hbc_lock
        .find_by_id(prod_ctx.plan.peer_doc_id.clone())
        .update(|mut_doc| {
            println!("---> producer set {} to {:?}", rec_path, rec);
            let mut_doc = mut_doc.unwrap();
            mut_doc
                .set(rec_path.as_str(), rec.clone())
                .expect("producer mutate doc");
        })
        .expect("producer mutate doc");
    prod_ctx.msg_index += 1;
}

pub fn producer_start(prod_ctx: ProducerCtx) -> JoinHandle<Result<u64, std::io::Error>> {
    println!("--> producer_start");
    let t = thread::spawn(move || -> Result<u64, std::io::Error> { producer_loop(prod_ctx) });
    t
}

pub fn producer_stop(prod_ctx: &ProducerCtx) {
    println!("--> producer_stop");
    prod_ctx
        .finished
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

// producer timer loop
pub fn producer_loop(mut prod_ctx: ProducerCtx) -> Result<u64, std::io::Error> {
    // TODO timing / message rate, etc.
    let mut count = 0;
    while !prod_ctx.finished.load(std::sync::atomic::Ordering::Relaxed) {
        producer_send(&mut prod_ctx);
        count += 1;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    Ok(count)
}
