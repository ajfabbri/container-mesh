use clap::Parser;
use common::types::*;
use common::util::*;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use env_logger::Env;
use log::*;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

mod producer;
use producer::*;
mod consumer;
use consumer::*;
mod context;
use context::*;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "container-mesh-coord")]
    coord_collection: String,

    #[arg(long)]
    coord_addr: String,

    #[arg(long, default_value_t = 4001)]
    coord_port: u32,

    #[arg(short, long)]
    bind_addr: Option<String>,

    #[arg(short = 'p', long)]
    bind_port: Option<u16>,

    #[arg(short, long, default_value = "peer")]
    device_name: String,

    #[arg(short, long, default_value = "/output")]
    output_dir: String,
}

fn make_ditto(device_name: &str) -> Result<Ditto, DittoError> {
    debug!("-> make_ditto");
    let make_id = |ditto_root| {
        let app_id = AppId::from_env("DITTO_APP_ID")?;
        identity::OfflinePlayground::new(ditto_root, app_id)
    };

    // Connect to ditto
    let ditto = Ditto::builder()
        .with_temp_dir()
        .with_minimum_log_level(LogLevel::Warning)
        .with_identity(make_id)?
        .build()
        .expect("ditto builder should succeed");
    ditto.set_device_name(device_name);
    Ok(ditto)
}

fn init_transport(pctx: &mut PeerContext, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut config = TransportConfig::default();
    config.peer_to_peer.lan.enabled = true;
    // fail fast
    let _ip_addr: std::net::IpAddr = cli.coord_addr.parse()?;
    let coord_addr = format!("{}:{}", cli.coord_addr, cli.coord_port);
    config.connect.tcp_servers = HashSet::from([coord_addr.clone()]);
    config.connect.websocket_urls = HashSet::new();
    config.listen.tcp.enabled = true;
    config.listen.tcp.interface_ip = pctx.local_ip.clone();
    config.listen.tcp.port = cli.bind_port.unwrap_or(0);
    info!(
        "-> set transport config {}:{}",
        config.listen.tcp.interface_ip, config.listen.tcp.port
    );
    debug!("-> config: {:?}", config);
    pctx.ditto.set_transport_config(config.clone());
    pctx.transport_config = Some(config);
    pctx.coord_addr = Some(coord_addr.clone());
    if log_enabled!(Level::Debug) {
        pctx.presence = Some(pctx.ditto.presence().observe(|pgraph| {
            debug!("--> presence update: {}", concise_presence(pgraph));
        }));
    }
    Ok(())
}

#[derive(Clone)]
pub struct HeartbeatCtx {
    peer_id: PeerId,
    record: Heartbeat,
    doc_id: DocumentId,
    // Could be an atomic usize w/ test and set as well
    state: Arc<Mutex<PeerState>>,
    finished: Arc<AtomicBool>,
    collection: Arc<Mutex<Collection>>,
}

// implement new
impl HeartbeatCtx {
    pub fn new(
        peer_id: PeerId,
        record: Heartbeat,
        doc_id: DocumentId,
        state: Arc<Mutex<PeerState>>,
        collection: Arc<Mutex<Collection>>,
    ) -> Self {
        HeartbeatCtx {
            peer_id,
            record,
            doc_id,
            state,
            finished: Arc::new(AtomicBool::new(false)),
            collection,
        }
    }
}

fn heartbeat_send(hctx: &mut HeartbeatCtx) {
    let hbc_lock = hctx.collection.lock().unwrap();
    hctx.record.update_timestamp();
    hctx.record.sender.state = hctx.state.lock().expect("lock hb ctx state").clone();
    debug!("---> heartbeat_send: update");
    hbc_lock
        .find_by_id(hctx.doc_id.clone())
        .update(|mut_doc| {
            let mut_doc = mut_doc.unwrap();
            mut_doc
                .set(
                    &vec!["beats", &hctx.peer_id.to_string()].join("."),
                    hctx.record.clone(),
                )
                .expect("mutate heartbeat doc");
        })
        .expect("update heartbeat doc");
}

fn heartbeat_start(hctx: HeartbeatCtx) -> JoinHandle<Result<(), std::io::Error>> {
    let t = thread::spawn(move || -> Result<(), std::io::Error> {
        heartbeat_loop(hctx)?;
        Ok(())
    });
    t
}

fn heartbeat_stop(hctx: &HeartbeatCtx) {
    hctx.finished
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

// heartbeat timer loop
fn heartbeat_loop(mut hctx: HeartbeatCtx) -> Result<(), std::io::Error> {
    // call heartbeat_send every second until done flag is set
    while !hctx.finished.load(std::sync::atomic::Ordering::Relaxed) {
        heartbeat_send(&mut hctx);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Ok(())
}

fn bootstrap_peer<'a>(pctx: &'a mut PeerContext, cli: &Cli) -> Result<(), Box<dyn Error>> {
    // subscribe to coordinator collection
    info!(
        "--> Subscribing to coordinator collection {}..",
        cli.coord_collection
    );
    debug!("-> init ditto");
    init_transport(pctx, &cli)?;
    pctx.ditto.set_license_from_env("DITTO_LICENSE")?;
    pctx.ditto.start_sync().expect("start_sync");

    let store = pctx.ditto.store();
    let coord_coll = store
        .collection(&cli.coord_collection)
        .expect("collection create");

    let _coord_sub = coord_coll.find_all().subscribe();
    // wait until we get an initial CoordinatorInfo
    let init_info;
    loop {
        debug!(
            "--> Polling for CoordinatorInfo on {:?}...",
            coord_coll.name()
        );
        match coord_coll.find_all().exec() {
            Err(e) => error!("Error: {:?}", e),
            Ok(plan) => {
                let n = plan.len();
                if n > 0 {
                    if n > 1 {
                        warn!(
                            "Warning: multiple coord info. documents (N={}), using the first.",
                            n
                        );
                    }
                    pctx.coord_doc_id = Some(plan[0].id());
                    init_info = Some(plan[0].typed::<CoordinatorInfo>()?);
                    debug!(
                        "--> got CoordinatorInfo id {}: {:?}",
                        pctx.coord_doc_id.as_ref().unwrap(),
                        init_info.as_ref().unwrap()
                    );
                    break;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    pctx.coord_info = init_info;
    let hb_record = Heartbeat {
        sender: Peer {
            state: PeerState::Init,
            peer_id: pctx.id.clone(),
            peer_ip_addr: pctx.local_ip.clone(),
            peer_port: pctx.local_port,
        },
        sent_at_usec: 0,
    };

    // Fetch initial heartbeat doc and start heartbeat timer
    let _hbc = store.collection(&pctx.coord_info.as_ref().unwrap().heartbeat_collection_name)?;
    let hbc = Arc::new(Mutex::new(_hbc));
    // retry until we get a heartbeat doc
    let hb_doc_id;
    let mut hb_sub: Option<Subscription> = None;
    loop {
        let hbc_lock = hbc.lock().unwrap();
        // lazy-init subscription
        if hb_sub.is_none() {
            hb_sub = Some(hbc_lock.find_all().subscribe());
        }
        let r = hbc_lock
            .find_all()
            .exec()
            .expect("Expected to find heartbeat doc");
        if r.len() < 1 {
            std::thread::sleep(std::time::Duration::from_secs(2));
        } else {
            if r.len() > 1 {
                warn!("Multiple heartbeat docs, using first.");
            }
            hb_doc_id = r[0].id();
            break;
        }
    }
    // No longer need to get heartbeat updates
    drop(hb_sub);

    let hctx = HeartbeatCtx::new(
        pctx.id.clone(),
        hb_record,
        hb_doc_id,
        pctx.state.clone(),
        hbc.clone(),
    );
    pctx.hb_thread = Some(heartbeat_start(hctx.clone()));
    pctx.hb_ctx = Some(hctx);

    // wait for execution plan
    info!("--> Waiting for execution plan..");
    loop {
        // XXX subscribe w/ callback instead of polling
        let doc_result = coord_coll
            .find_by_id(pctx.coord_doc_id.as_ref().unwrap())
            .exec();
        if let Err(e) = doc_result {
            warn!("Error finding doc in coord. collection: {:?}", e);
            continue;
        }
        if let Ok(bd) = doc_result {
            let coord_info = bd.typed::<CoordinatorInfo>()?;
            debug!("---> Got CoordinatorInfo: {:?}", coord_info);
            pctx.coord_info = Some(coord_info);
            if pctx.coord_info.as_ref().unwrap().execution_plan.is_some() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    debug!("Got execution plan {:?}", pctx.coord_info);
    Ok(())
}

fn connect_mesh(pctx: &PeerContext) -> Result<(), Box<dyn Error>> {
    // Connect to all other peers in coord_info.
    let mut all_peers = pctx
        .transport_config
        .as_ref()
        .unwrap()
        .connect
        .tcp_servers
        .clone();
    let n = all_peers.len();
    let plan = pctx.get_plan().unwrap();
    let my_peers = plan.connections.nmap.get(&pctx.id);
    for p in my_peers.unwrap() {
        let peer_obj = plan.peers.iter().find(|x| x.peer_id == *p).unwrap();
        all_peers.insert(format!("{}:{}", peer_obj.peer_ip_addr, peer_obj.peer_port));
    }
    let m = all_peers.len();
    info!("--> connect_mesh: num peers {} -> {}", n, m);
    let mut new_config = pctx.transport_config.as_ref().unwrap().clone();
    new_config.connect.tcp_servers = all_peers;
    debug!("--> set transport config: {:?}", new_config);
    pctx.ditto.set_transport_config(new_config);
    Ok(())
}

fn run_test(pctx: &mut PeerContext) -> Result<PeerReport, Box<dyn Error>> {
    // connect to all other peers in coord_info
    connect_mesh(pctx)?;

    // wait for start time
    let plan = pctx.get_plan().unwrap();
    let start_time = plan.start_time;
    let now = system_time_msec();
    // XXX TODO can underflow
    let wait_time;
    if now > start_time {
        wait_time = 0;
    } else {
        wait_time = start_time - now;
    }
    info!("--> Waiting {} msec for start time", wait_time);
    std::thread::sleep(std::time::Duration::from_millis(wait_time as u64));

    pctx.state_transition(Some(PeerState::Init), PeerState::Running)?;

    // set up message processor that processes changes to peer collection
    let cc = consumer_create_collection(pctx)?;
    pctx.peer_collection = Some(Arc::new(Mutex::new(cc)));
    let _consumer = consumer_start(pctx)?;

    // Send messages at desired rates
    let producer = ProducerCtx::new(
        pctx.id.clone(),
        pctx.peer_collection.as_ref().unwrap().clone(),
        plan.clone(),
    );

    let _pthread = producer_start(producer.clone());

    // wait for test duration
    info!(
        "--> Waiting {} sec for test duration",
        plan.test_duration_sec
    );
    thread::sleep(Duration::from_secs(plan.test_duration_sec as u64));
    debug!("--> Shutting down producer..");
    producer_stop(&producer);

    let msg_count = _pthread.join().unwrap().unwrap();
    pctx.state_transition(Some(PeerState::Running), PeerState::Reporting)?;

    // Return test report
    let consumer = _consumer.lock().unwrap();
    let _stats = LatencyStats::new();
    let report = PeerReport {
        message_latency: consumer.get_message_latency(),
        records_produced: msg_count,
    };
    pctx.state_transition(Some(PeerState::Reporting), PeerState::Shutdown)?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    Ok(report)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut pctx = PeerContext::new(
        &cli.device_name,
        make_ditto(&cli.device_name)?,
        resolve_local_ip(cli.bind_addr.clone()).as_str(),
        cli.bind_port.unwrap(),
    );
    debug!("Args {:?}", cli);
    bootstrap_peer(&mut pctx, &cli)?;

    info!("--> Running test plan..");
    let report = run_test(&mut pctx)?;
    let fname = PathBuf::from(format!(
        "{}/{}-report.json",
        &cli.output_dir, &cli.device_name
    ));

    info!(
        "--> Test report (saving to {}): {:?}",
        fname.to_str().unwrap(),
        report
    );

    // write report to file
    let mut f = File::create(fname)?;
    f.write_all(format!("{:?}", report).as_bytes())?;
    drop(f);

    // shutdown
    heartbeat_stop(pctx.hb_ctx.as_ref().unwrap());

    Ok(())
}
