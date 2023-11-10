use clap::Parser;
use common::default::HEARTBEAT_COLLECTION_NAME;
use common::types::*;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "container-mesh-coord")]
    coord_collection: String,

    #[arg(long, default_value_t = 1)]
    min_peers: u32,

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
    id: u64,
    ditto: Ditto,
    plan: Option<ExecutionPlan>,
    start_time_msec: u64,
    coord_collection: Option<Collection>,
    coord_doc_id: Option<DocumentId>,
    hb_collection: Option<Collection>,
    hb_doc_id: Option<DocumentId>,
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

// XXX TODO just use a tuple?
struct HeartbeatProcessor {
    peer_set: Arc<Mutex<HashSet<Peer>>>,
    added: Condvar,
}

impl HeartbeatProcessor {
    fn process_heartbeat(&self, hbd: HeartbeatsDoc) {
        println!("--> process {} peer heartbeats", hbd.beats.len());
        for hb in hbd.beats {
            println!("--> got heartbeat {:?}", hb);
            let mut peer_set = self.peer_set.lock().unwrap();
            peer_set.insert(hb.sender);
        }
    }
}

fn update_coord_info(
    cc: &Collection,
    plan: Option<ExecutionPlan>,
) -> Result<DocumentId, DittoError> {
    println!("XXX -> update_coord_info for {:?}", cc.name());
    // Upsert is ok since this is infrequently updated
    let mut ci = CoordinatorInfo::default();
    ci.execution_plan = plan;
    cc.upsert(ci)
}

fn wait_for_quorum(
    ctx: &mut CoordinatorContext,
    coord_collection: &str,
    min_peers: u32,
) -> Result<(), DittoError> {
    let store = ctx.ditto.store();

    // Set up presence monitoring so we can tell if peers are connecting
    let _presence = ctx.ditto.presence().observe( |graph| {
        println!("XXX -> presence update {:?}", graph);
    });

    // Populate coord. collection with initial info.
    ctx.coord_collection = Some(store.collection(coord_collection)?);
    // TODO assert collection is empty
    ctx.coord_doc_id = Some(update_coord_info(
        ctx.coord_collection.as_ref().unwrap(),
        None,
    )?);
    println!(
        "XXX --> wrote coord info doc id {}",
        ctx.coord_doc_id.as_ref().unwrap()
    );

    // Set up heartbeats document and  consumer
    println!("XXX -> create empty heartbeats doc");
    let hbc = store.collection(HEARTBEAT_COLLECTION_NAME)?;
    ctx.hb_doc_id = Some(hbc.upsert(HeartbeatsDoc { beats: Vec::new() })?);
    ctx.hb_collection = Some(hbc);

    println!("XXX -> set up heartbeat consumer");
    let coll = ctx.hb_collection.as_ref().unwrap();
    let hbp = Arc::new(HeartbeatProcessor {
        peer_set: Arc::clone(&ctx.peers),
        added: Condvar::new(),
    });
    let cb = hbp.clone();
    let _peer_observer = coll
        .find_all()
        .observe_local(move |docs: Vec<BoxedDocument>, event| {
            println!(
                "XXX -> observe_local event {:?} with N={}",
                event,
                docs.len()
            );
            docs.iter().for_each(|doc| {
                let r = doc.typed::<HeartbeatsDoc>();
                match r {
                    Ok(hb) => cb.process_heartbeat(hb),
                    Err(e) => {
                        println!("Heartbeat deser Error {:?}", e);
                        let p = doc.to_cbor().unwrap();
                        println!("Received heartbeat cbor {:?}", p);
                    }
                }
            });
        })
        .unwrap();

    loop {
        let peers = hbp.peer_set.lock().unwrap();
        let n: u32 = peers.len().try_into().unwrap();
        if n >= min_peers {
            println!(
                "Have {} peers, waiting 5 seconds then attempting to start...",
                n
            );
            drop(peers);
            thread::sleep(Duration::from_secs(5));
            break;
        }
        println!("Waiting for peers (have {} of at least {})", n, min_peers);
        let _unused = hbp.added.wait(peers).unwrap();
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    println!("Args {:?}", cli);
    let mut ctx = CoordinatorContext {
        id: 0,
        ditto: make_ditto()?,
        plan: None,
        start_time_msec: 0,
        coord_collection: None,
        coord_doc_id: None,
        hb_collection: None,
        hb_doc_id: None,
        peers: Arc::new(Mutex::new(HashSet::new())),
    };
    println!("XXX -> init ditto");
    init_transport(&mut ctx, &cli)?;
    ctx.ditto.set_license_from_env("DITTO_LICENSE")?; ctx.ditto.start_sync()?;

    println!("XXX -> wait for quorum");
    wait_for_quorum(&mut ctx, &cli.coord_collection, cli.min_peers)?;
    Ok(())
}
