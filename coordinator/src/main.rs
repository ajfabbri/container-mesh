use clap::Parser;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::Duration;
use common::types::*;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "container-mesh-coord")]
    coordinator_collection: String,

    #[arg(short, long, default_value_t = 1)]
    min_peers: u32,

    #[arg(short, long, default_value_t = 10)]
    min_msg_delay_msec: u32,

    #[arg(short, long, default_value_t = 500)]
    max_msg_delay_msec: u32,

    #[arg(short, long, default_value_t = 60)]
    test_duration_sec: u32,

    #[arg(short, long, default_value = "0.0.0.0")]
    bind_addr: String,

    #[arg(short, long, default_value_t = 4001)]
    bind_port: u32,
}

struct CoordinatorContext {
    id: u64,
    ditto: Ditto,
    plan: Option<ExecutionPlan>,
    start_time_msec: u64,
    coord_collection: Option<Collection>,
    peers: Arc<Mutex<HashSet<Peer>>>,
}

fn make_ditto() -> Result<Ditto, DittoError> {
    let make_id = |ditto_root| {
        let app_id = AppId::from_env("DITTO_APP_ID")?;
        let shared_token = std::env::var("DITTO_PG_TOKEN").unwrap();
        let cloud_sync = true;
        let custom_auth_url = None;
        identity::OnlinePlayground::new(
            ditto_root,
            app_id,
            shared_token,
            cloud_sync,
            custom_auth_url,
        )
    };

    // Connect to ditto
    let ditto = Ditto::builder()
        .with_root(Arc::new(
            PersistentRoot::from_current_exe().expect("Invalid Ditto Root"),
        ))
        .with_minimum_log_level(LogLevel::Info)
        .with_identity(make_id)?
        .build()
        .expect("ditto builder should succeed");
    Ok(ditto)
}

fn init_transport(ctx: &mut CoordinatorContext, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut config = TransportConfig::new();
    config.enable_all_peer_to_peer();
    config.connect.tcp_servers = HashSet::new();
    config.connect.websocket_urls = HashSet::new();
    config.listen.tcp.enabled = true;
    config.listen.tcp.interface_ip = cli.bind_addr.clone();
    config.listen.tcp.port = cli.bind_port.try_into()?;
    ctx.ditto.set_transport_config(config);
    Ok(())
}

// XXX TODO just use a tuple?
struct HeartbeatProcessor {
    peer_set: Arc<Mutex<HashSet<Peer>>>,
    added: Condvar,
}

impl HeartbeatProcessor {
    fn process_heartbeat(&self, hb: Heartbeat) {
        println!("Got heartbeat {:?}", hb);
        let mut peer_set = self.peer_set.lock().unwrap();
        peer_set.insert(hb.sender);
    }
}

fn wait_for_quorum(ctx: &mut CoordinatorContext, coord_collection: &str, min_peers: u32) ->
Result<(), DittoError> {
    let store = ctx.ditto.store();
    ctx.coord_collection = Some(store.collection(coord_collection)?);
    ctx.ditto.start_sync()?;

    let coll = ctx.coord_collection.as_ref().unwrap();
    let hbp = Arc::new( HeartbeatProcessor {
        peer_set: Arc::clone(&ctx.peers),
        added: Condvar::new(),
    });
    let cb = hbp.clone();
    let _peer_observer = coll
        .find_all()
        .observe_local(move |docs: Vec<BoxedDocument>, event| {
            docs.iter().for_each(|doc| {
                let val = doc.typed::<Heartbeat>().unwrap();
                cb.process_heartbeat(val);
            });
            println!("Got event {:?}", event);
        }).unwrap();

    loop {
        let peers = hbp.peer_set.lock().unwrap();
        let n: u32 = peers.len().try_into().unwrap();
        if n >=  min_peers {
            println!("Have {} peers, waiting 5 seconds then attempting to start...", n);
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
        peers: Arc::new(Mutex::new(HashSet::new())),
    };
    init_transport(&mut ctx, &cli);

    wait_for_quorum(&mut ctx, &cli.coordinator_collection, cli.min_peers)?;
    Ok(())
}
