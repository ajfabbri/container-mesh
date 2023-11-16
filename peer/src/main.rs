use clap::Parser;
use common::types::*;
use common::util::*;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;

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
    bind_port: Option<u32>,

    #[arg(short, long, default_value = "peer")]
    device_name: String,
}

fn make_ditto(device_name: &str) -> Result<Ditto, DittoError> {
    println!("-> make_ditto");
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
    };

    // Connect to ditto
    let ditto = Ditto::builder()
        .with_temp_dir()
        // .with_root(Arc::new(
        //     PersistentRoot::from_current_exe().expect("Invalid Ditto Root"),
        // ))
        .with_minimum_log_level(LogLevel::Info)
        .with_identity(make_id)?
        .build()
        .expect("ditto builder should succeed");
    ditto.set_device_name(device_name);
    Ok(ditto)
}

fn init_transport(pctx: &PeerContext, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut config = TransportConfig::default();
    config.peer_to_peer.lan.enabled = true;
    // fail fast
    let _ip_addr: std::net::IpAddr = cli.coord_addr.parse()?;
    config.connect.tcp_servers = HashSet::from([format!("{}:{}", cli.coord_addr, cli.coord_port)]);
    config.connect.websocket_urls = HashSet::new();
    config.listen.tcp.enabled = true;
    config.listen.tcp.interface_ip = pctx.local_ip.clone();
    if cli.bind_port.is_some() {
        config.listen.tcp.port = cli.bind_port.unwrap_or(0).try_into()?;
    }
    println!(
        "-> set transport config {}:{}",
        config.listen.tcp.interface_ip, config.listen.tcp.port
    );
    println!("XXX --> config: {:?}", config);
    pctx.ditto.set_transport_config(config);
    Ok(())
}

#[derive(Clone)]
struct HeartbeatCtx {
    peer_id: PeerId,
    record: Heartbeat,
    doc_id: DocumentId,
    // Could be an atomic usize w/ test and set as well
    state: Arc<Mutex<PeerState>>,
    // TODO fold this into `state`?
    finished: Arc<AtomicBool>,
    hb_collection: Arc<Mutex<Collection>>,
}

struct PeerContext {
    id: PeerId,
    ditto: Ditto,
    coord_doc_id: Option<DocumentId>,
    coord_info: Option<CoordinatorInfo>,
    #[allow(dead_code)]
    hb_doc_id: Option<DocumentId>,
    hb_collection: Option<Arc<Mutex<Collection>>>,
    hb_subscription: Option<Subscription>,
    #[allow(dead_code)]
    start_time_msec: u64,
    local_ip: String,
    state: Arc<Mutex<PeerState>>,
}

// implement new
impl HeartbeatCtx {
    fn new(
        peer_id: PeerId,
        record: Heartbeat,
        doc_id: DocumentId,
        state: Arc<Mutex<PeerState>>,
        hb_collection: Arc<Mutex<Collection>>,
    ) -> Self {
        HeartbeatCtx {
            peer_id,
            record,
            doc_id,
            state,
            finished: Arc::new(AtomicBool::new(false)),
            hb_collection,
        }
    }
}

fn heartbeat_send(hctx: &mut HeartbeatCtx) {
    println!("---> heartbeat_send");
    let hbc_lock = hctx.hb_collection.lock().unwrap();
    hctx.record.update_timestamp();
    hctx.record.sender.state = hctx.state.lock().expect("lock hb ctx state").clone();
    println!("---> heartbeat_send: update");
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

fn heartbeat_start(hctx: HeartbeatCtx) -> Result<(), std::io::Error> {
    let t = thread::spawn(move || -> Result<(), std::io::Error> {
        heartbeat_loop(hctx)?;
        Ok(())
    });
    t.join().unwrap()?;
    Ok(())
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

fn bootstrap_peer<'a>(
    pctx: &'a mut PeerContext,
    cli: &Cli,
) -> Result<ExecutionPlan, Box<dyn Error>> {
    // subscribe to coordinator collection
    println!(
        "--> Subscribing to coordinator collection {}..",
        cli.coord_collection
    );
    println!("-> init ditto");
    init_transport(&pctx, &cli)?;
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
        println!(
            "--> Polling for CoordinatorInfo on {:?}...",
            coord_coll.name()
        );
        match coord_coll.find_all().exec() {
            Err(e) => println!("Error: {:?}", e),
            Ok(plan) => {
                let n = plan.len();
                if n > 0 {
                    if n > 1 {
                        println!(
                            "Warning: multiple coord info. documents (N={}), using the first.",
                            n
                        );
                    }
                    pctx.coord_doc_id = Some(plan[0].id());
                    init_info = Some(plan[0].typed::<CoordinatorInfo>()?);
                    println!(
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
        },
        sent_at_msec: 0,
    };

    // Fetch initial heartbeat doc and start heartbeat timer
    let _hbc = store.collection(&pctx.coord_info.as_ref().unwrap().heartbeat_collection_name)?;
    let hbc = Arc::new(Mutex::new(_hbc));
    // retry until we get a heartbeat doc
    let hb_doc_id;
    loop {
        let hbc_lock = hbc.lock().unwrap();
        // lazy-init context's heartbeat subscription and collection
        if pctx.hb_subscription.is_none() {
            pctx.hb_subscription = Some(hbc_lock.find_all().subscribe());
            pctx.hb_collection = Some(hbc.clone());
        }
        let r = hbc_lock
            .find_all()
            .exec()
            .expect("Expected to find heartbeat doc");
        if r.len() < 1 {
            std::thread::sleep(std::time::Duration::from_secs(2));
        } else {
            if r.len() > 1 {
                println!("Warning: multiple heartbeat docs, using first.");
            }
            hb_doc_id = r[0].id();
            break;
        }
    }
    let hctx = HeartbeatCtx::new(
        pctx.id.clone(),
        hb_record,
        hb_doc_id,
        pctx.state.clone(),
        hbc,
    );
    heartbeat_start(hctx.clone())?;

    // wait for execution plan
    loop {
        // XXX subscribe w/ callback instead of polling
        let doc_result = coord_coll
            .find_by_id(pctx.coord_doc_id.as_ref().unwrap())
            .exec();
        if let Err(e) = doc_result {
            println!("Error finding doc in coord. collection: {:?}", e);
            continue;
        }
        if let Ok(bd) = doc_result {
            let coord_info = bd.typed::<CoordinatorInfo>()?;
            pctx.coord_info = Some(coord_info);
            if pctx.coord_info.as_ref().unwrap().execution_plan.is_some() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    println!("Got execution plan {:?}", pctx.coord_info);

    // TODO pass in and stop at end of execution
    heartbeat_stop(&hctx);
    todo!()
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let mut pctx = PeerContext {
        id: random_peer_id(Some(&cli.device_name)),
        coord_doc_id: None,
        ditto: make_ditto(&cli.device_name)?,
        coord_info: None,
        hb_doc_id: None,
        hb_collection: None,
        hb_subscription: None,
        start_time_msec: system_time_msec(),
        local_ip: resolve_local_ip(cli.bind_addr.clone()),
        state: Arc::new(Mutex::new(PeerState::Init)),
    };
    println!("Args {:?}", cli);
    bootstrap_peer(&mut pctx, &cli)?;
    Ok(())
}
