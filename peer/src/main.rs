use clap::Parser;
use common::types::*;
use common::util::*;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "container-mesh-coord")]
    coord_collection: String,

    #[arg(long)]
    coord_addr: String,

    #[arg(long, default_value_t = 4001)]
    coord_port: u32,

    #[arg(short, long, default_value = "0.0.0.0")]
    bind_addr: String,

    #[arg(short = 'p', long, default_value_t = 4001)]
    bind_port: u32,

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

fn init_transport(ditto: &mut Ditto, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut config = TransportConfig::default();
    config.peer_to_peer.lan.enabled = true;
    // fail fast
    let _ip_addr: std::net::IpAddr = cli.coord_addr.parse()?;
    config.connect.tcp_servers = HashSet::from([format!("{}:{}", cli.coord_addr, cli.coord_port)]);
    config.connect.websocket_urls = HashSet::new();
    config.listen.tcp.enabled = true;
    config.listen.tcp.interface_ip = cli.bind_addr.clone();
    config.listen.tcp.port = cli.bind_port.try_into()?;
    println!(
        "-> set transport config {}:{}",
        config.listen.tcp.interface_ip, config.listen.tcp.port
    );
    println!("XXX --> config: {:?}", config);
    ditto.set_transport_config(config);
    Ok(())
}

#[derive(Clone)]
struct HeartbeatCtx {
    peer_id: u64,
    record: Heartbeat,
    doc: HeartbeatsDoc,
    finished: Arc<AtomicBool>,
    collection: Collection,
}

struct PeerContext {
    id: u64,
    ditto: Ditto,
    coord_doc_id: Option<DocumentId>,
    coord_info: Option<CoordinatorInfo>,
    hb_doc_id: Option<DocumentId>,
    start_time_msec: u64,
}

// implement new
impl HeartbeatCtx {
    fn new(peer_id: u64, record: Heartbeat, doc: HeartbeatsDoc, collection: Collection) -> Self {
        HeartbeatCtx {
            peer_id,
            record,
            doc,
            finished: Arc::new(AtomicBool::new(false)),
            collection,
        }
    }
}

fn heartbeat_send(hctx: &mut HeartbeatCtx) {
    // find our id in the doc and update it
    // TODO may not be the most Ditto-idiomatic way to do this..
    let mut found = false;
    for hb in hctx.doc.beats.iter_mut() {
        if hb.sender.peer_id == hctx.peer_id {
            hb.sent_at_msec = system_time_msec();
            found = true;
            break;
        }
    }
    if !found {
        println!("Adding self (id={}) to heartbeat doc", hctx.peer_id);
        hctx.record.update_timestamp();
        hctx.doc.beats.push(hctx.record.clone());
    }
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
    init_transport(&mut pctx.ditto, &cli)?;
    pctx.ditto.set_license_from_env("DITTO_LICENSE")?;
    pctx.ditto.start_sync().expect("start_sync");

    let store = pctx.ditto.store();
    let collection = store
        .collection(&cli.coord_collection)
        .expect("collection create");

    let coord_sub = coord_coll.find_all().subscribe();
    // wait until we get an initial CoordinatorInfo
    let init_info;
    loop {
        println!(
            "--> Polling for CoordinatorInfo on {:?}...",
            collection.name()
        );
        match collection.find_all().exec() {
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
                        "Got CoordinatorInfo id {:?}: {:?}",
                        pctx.coord_doc_id,
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
            peer_id: pctx.id,
            peer_ip_addr: std::net::IpAddr::V4(cli.bind_addr.parse()?),
        },
        sent_at_msec: 0,
    };

    // Fetch heartbeat doc and start heartbeat timer
    let hbc = store.collection(&pctx.coord_info.as_ref().unwrap().heartbeat_collection_name)?;
    let r = hbc
        .find_all()
        .exec()
        .expect("Expected to find heartbeat doc");
    if r.len() < 1 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No heartbeat doc found",
        )));
    }
    if r.len() > 1 {
        println!("Warning: duplicate heartbeat docs found, using first.");
    }
    let hb_doc = r[0].typed::<HeartbeatsDoc>()?;
    let hctx = HeartbeatCtx::new(pctx.id, hb_record, hb_doc, hbc);
    heartbeat_start(hctx.clone())?;

    // wait for execution plan
    loop {
        // XXX subscribe w/ callback instead of polling
        let doc_result = collection
            .find_by_id(pctx.coord_doc_id.as_ref().unwrap())
            .exec();
        if let Err(e) = doc_result {
            println!("Error finding doc in coord. collection: {:?}", e);
            continue;
        }
        if let Ok(plan) = doc_result {
            let foo = plan.typed::<CoordinatorInfo>()?;
            pctx.coord_info = Some(foo);
            break;
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
        id: rand::random::<u64>(),
        coord_doc_id: None,
        ditto: make_ditto(&cli.device_name)?,
        coord_info: None,
        hb_doc_id: None,
        start_time_msec: system_time_msec(),
    };
    println!("Args {:?}", cli);
    bootstrap_peer(&mut pctx, &cli)?;
    Ok(())
}
