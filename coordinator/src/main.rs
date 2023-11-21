use clap::Parser;
use common::default::*;
use common::types::PeerState::*;
use common::types::*;
use common::util::*;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "container-mesh-coord")]
    coord_collection: String,

    #[arg(long, default_value_t = 1)]
    min_peers: usize,

    #[arg(long, default_value_t = 10)]
    min_msg_delay_msec: u32,

    #[arg(long, default_value_t = 500)]
    max_msg_delay_msec: u32,

    #[arg(short = 'd', long, default_value_t = 60)]
    test_duration_sec: u32,

    #[arg(short, long, default_value = "0.0.0.0")]
    bind_addr: String,

    #[arg(short = 'p', long, default_value_t = 4001)]
    bind_port: u32,
}

struct CoordinatorContext {
    ditto: Ditto,
    plan: Option<ExecutionPlan>,
    start_time_msec: u64,
    coord_collection: Option<Collection>,
    coord_doc_id: Option<DocumentId>,
    hb_collection: Option<Collection>,
    hb_doc_id: Option<DocumentId>,
    hb_processor: Option<Arc<HeartbeatProcessor>>,
    hb_observer: Option<LiveQuery>,
    peers: Arc<Mutex<HashSet<Peer>>>,
}

fn make_ditto() -> Result<Ditto, DittoError> {
    println!("XXX -> make_ditto");
    let make_id = |ditto_root| {
        let app_id = AppId::from_env("DITTO_APP_ID")?;
        identity::OfflinePlayground::new(ditto_root, app_id)
        //let shared_token = std::env::var("DITTO_PG_TOKEN").unwrap();
        //let cloud_sync = true;
        //let custom_auth_url = None;
        //identity::OnlinePlayground::new(
        //    ditto_root,
        //    app_id,
        //    shared_token,
        //    cloud_sync,
        //    custom_auth_url,
        //)
    };

    println!("XXX -> make_ditto -> builder");
    // Connect to ditto
    let ditto = Ditto::builder()
        .with_temp_dir()
        //.with_root(Arc::new(
        //    PersistentRoot::from_current_exe().expect("Invalid Ditto Root"),
        //))
        .with_minimum_log_level(LogLevel::Info)
        .with_identity(make_id)?
        .build()
        .expect("ditto builder should succeed");
    ditto.set_device_name("coordinator");
    Ok(ditto)
}

fn init_transport(ctx: &mut CoordinatorContext, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut config = TransportConfig::default();
    config.peer_to_peer.lan.enabled = true;
    // initialize peer set with coordinator's address and port
    config.connect.tcp_servers = HashSet::new();
    config.connect.websocket_urls = HashSet::new();
    config.listen.tcp.enabled = true;
    config.listen.tcp.interface_ip = cli.bind_addr.clone();
    config.listen.tcp.port = cli.bind_port.try_into()?;
    println!(
        "XXX -> set transport config {}:{}",
        config.listen.tcp.interface_ip, config.listen.tcp.port
    );
    println!("XXX --> config: {:?}", config);
    ctx.ditto.set_transport_config(config);
    Ok(())
}

struct HeartbeatProcessor {
    peer_set: Arc<Mutex<HashSet<Peer>>>,
    added: Condvar,
    // To keep subscription alive as needed
    #[allow(dead_code)]
    subscription: Subscription,
}

impl HeartbeatProcessor {
    fn process_heartbeat(&self, hbd: HeartbeatsDoc) {
        println!("--> process {} peer heartbeats", hbd.beats.len());
        for (_peer_id, hb) in hbd.beats {
            println!("--> got heartbeat {:?}", hb);
            let mut peer_set = self.peer_set.lock().unwrap();
            peer_set.insert(hb.sender);
            println!("--> peer set: {:?}", peer_set);
            self.added.notify_all();
        }
    }
}

fn upsert_coord_info(
    cc: &Collection,
    plan: Option<ExecutionPlan>,
) -> Result<DocumentId, DittoError> {
    let mut ci = CoordinatorInfo::default();
    ci.execution_plan = plan;
    cc.upsert(ci)
}

fn set_coord_info_plan(
    cc: &Collection,
    cid: DocumentId,
    plan: ExecutionPlan,
) -> Result<(), DittoError> {
    println!("XXX -> update_coord_info for {:?}", cc.name());
    let _res = cc.find_by_id(cid).update(|mut_doc| {
        let mut_doc = mut_doc.unwrap();
        mut_doc
            .set("execution_plan", plan.clone())
            .expect("set execution_plan");
    })?;
    Ok(())
}

fn init_coord_collection(
    ctx: &mut CoordinatorContext,
    coord_collection: &str,
) -> Result<(), Box<dyn Error>> {
    let store = ctx.ditto.store();

    // Populate coord. collection with initial info.
    ctx.coord_collection = Some(store.collection(coord_collection)?);
    // TODO assert collection is empty
    ctx.coord_doc_id = Some(upsert_coord_info(
        ctx.coord_collection.as_ref().unwrap(),
        None,
    )?);
    println!(
        "XXX --> wrote coord info doc id {}",
        ctx.coord_doc_id.as_ref().unwrap()
    );
    Ok(())
}

fn init_heartbeat_processor(ctx: &mut CoordinatorContext) -> Result<(), Box<dyn Error>> {
    // Set up heartbeats document and  consumer
    println!("XXX -> create empty heartbeats doc and subscribing");
    let store = ctx.ditto.store();
    let hbc = store.collection(HEARTBEAT_COLLECTION_NAME)?;
    ctx.hb_doc_id = Some(hbc.upsert(HeartbeatsDoc {
        beats: HashMap::new(),
    })?);
    ctx.hb_collection = Some(hbc);

    println!("XXX -> set up heartbeat consumer");
    let hb_coll = ctx.hb_collection.as_ref().unwrap();
    let hb_query = hb_coll.find_by_id(ctx.hb_doc_id.as_ref().unwrap());
    let _hb_sub = hb_query.subscribe();
    let cb = Arc::new(HeartbeatProcessor {
        peer_set: Arc::clone(&ctx.peers),
        added: Condvar::new(),
        subscription: _hb_sub,
    });
    ctx.hb_processor = Some(cb.clone());
    ctx.hb_observer = Some(
        hb_query
            .observe_local(move |doc: Option<BoxedDocument>, event| {
                println!("XXX -> observe_local event {:?}", event);
                if doc.is_none() {
                    return;
                }
                let r = doc.as_ref().unwrap().typed::<HeartbeatsDoc>();
                match r {
                    Ok(hb) => {
                        println!("OK received heartbeat:");
                        //let p = doc.unwrap().to_cbor().unwrap();
                        //print_cdoc(&p).unwrap();
                        cb.process_heartbeat(hb);
                    }
                    Err(e) => {
                        println!("Heartbeat deser Error {:?}", e);
                        let p = doc.unwrap().to_cbor().unwrap();
                        println!("received heartbeat:");
                        print_cdoc(&p).unwrap();
                    }
                }
            })
            .unwrap(),
    );
    Ok(())
}

fn wait_for_quorum(
    ctx: &mut CoordinatorContext,
    coord_collection: &str,
    min_peers: usize,
) -> Result<(), Box<dyn Error>> {
    init_coord_collection(ctx, coord_collection)?;
    init_heartbeat_processor(ctx)?;
    wait_for_peer_state(ctx.hb_processor.as_ref().unwrap(), Init, min_peers)
}

fn wait_for_peer_state(
    hbp: &HeartbeatProcessor,
    state: PeerState,
    min_peers: usize,
) -> Result<(), Box<dyn Error>> {
    wait_for_peer_states(hbp, vec![state], min_peers)
}

fn wait_for_peer_states(
    hbp: &HeartbeatProcessor,
    states: Vec<PeerState>,
    min_peers: usize,
) -> Result<(), Box<dyn Error>> {
    loop {
        println!("XXX -> wait for peers");
        let peers = hbp.peer_set.lock().unwrap();
        // total of n peers, k of which are in desired `state`
        let n: usize = peers.len();
        let k = peers.iter().filter(|p| states.contains(&p.state)).count();
        println!("XXX -> have {} peers, {} in state(s) {:?}", n, k, states);
        if k >= min_peers {
            drop(peers);
            break;
        } else {
            println!(
                "Waiting for peers (k = {}, n = {}, need {})",
                k, n, min_peers
            );
            let _unused = hbp.added.wait(peers);
        }
    }
    Ok(())
}

fn generate_plan(_ctx: &CoordinatorContext) -> ExecutionPlan {
    let mut plan = ExecutionPlan::default();
    plan.test_duration_sec = 10;
    plan
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    println!("Args {:?}", cli);
    let mut ctx = CoordinatorContext {
        ditto: make_ditto()?,
        plan: None,
        start_time_msec: 0,
        coord_collection: None,
        coord_doc_id: None,
        hb_collection: None,
        hb_doc_id: None,
        hb_processor: None,
        hb_observer: None,
        peers: Arc::new(Mutex::new(HashSet::new())),
    };
    println!("XXX -> init ditto");
    init_transport(&mut ctx, &cli)?;
    ctx.ditto.set_license_from_env("DITTO_LICENSE")?;
    ctx.ditto.start_sync()?;

    println!("XXX -> wait for quorum");
    wait_for_quorum(&mut ctx, &cli.coord_collection, cli.min_peers)?;
    println!("XXX -> got quorum, writing test plan..");
    let plan = generate_plan(&ctx);
    set_coord_info_plan(
        ctx.coord_collection.as_ref().unwrap(),
        ctx.coord_doc_id.unwrap(),
        plan,
    )?;

    println!("XXX -> waiting for peers to start Running..");
    wait_for_peer_state(ctx.hb_processor.as_ref().unwrap(), Running, cli.min_peers)?;

    println!("XXX --> waiting for peers to finish running..");
    wait_for_peer_states(
        ctx.hb_processor.as_ref().unwrap(),
        vec![Reporting, Shutdown],
        cli.min_peers,
    )?;

    Ok(())
}
