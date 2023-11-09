use clap::Parser;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use chrono::prelude::*;
use std::error::Error;
use std::collections::HashSet;
use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time;
use common::types::*;
use common::util::*;


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

    #[arg(short='p', long, default_value_t = 4001)]
    bind_port: u32,
}

fn make_ditto() -> Result<Ditto, DittoError> {
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
    Ok(ditto)
}

fn init_transport(ditto: &mut Ditto, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut config = TransportConfig::new();
    config.enable_all_peer_to_peer();
    let _ip_addr: std::net::IpAddr = cli.coord_addr.parse()?;
    config.connect.tcp_servers = HashSet::from([cli.coord_addr.clone()]);
    config.listen.tcp.interface_ip = cli.bind_addr.clone();
    config.listen.tcp.port = cli.bind_port.try_into()?;
    ditto.set_transport_config(config);
    Ok(())
}

#[allow(dead_code)]
fn print_cdoc(cbor: &serde_cbor::Value) -> Result<(), io::Error> {
    serde_json::to_writer_pretty(std::io::stdout(), cbor)?;
    println!();
    Ok(())
}

#[derive(Clone)]
struct HeartbeatCtx {
    record: Heartbeat,
    finished: Arc<AtomicBool>,
}

struct PeerContext {
    id: u64,
    ditto: Ditto,
    coord_info: Option<CoordinatorInfo>,
    start_time_msec: u64,
}

// implement new
impl HeartbeatCtx {
    fn new(hb: Heartbeat) -> Self {
        HeartbeatCtx {
            record: hb,
            finished: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn heartbeat_send(_hctx: &HeartbeatCtx) {
    println!("TODO send_heartbeat at {}", Utc::now());
    println!("           system time {:?}", time::SystemTime::now());
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
    hctx.finished.store(true, std::sync::atomic::Ordering::Relaxed);
}

// heartbeat timer loop
fn heartbeat_loop(hctx: HeartbeatCtx) -> Result<(), std::io::Error> {
    // call heartbeat_send every second until done flag is set
    while !hctx.finished.load(std::sync::atomic::Ordering::Relaxed) {
        heartbeat_send(&hctx);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Ok(())
}

fn bootstrap_peer<'a>(pctx: &'a mut PeerContext, cli: &Cli) -> Result<ExecutionPlan, Box<dyn Error>> {
    // subscribe to coordinator collection
    println!("Subscribing to coordinator collection {}..", cli.coord_collection);
    pctx.ditto = make_ditto().expect("make_ditto");
    init_transport(&mut pctx.ditto, &cli)?;
    let store = pctx.ditto.store();
    let collection = store
        .collection(&cli.coord_collection)
        .expect("collection create");
    pctx.ditto.set_license_from_env("DITTO_LICENSE")?;
    pctx.ditto.start_sync().expect("start_sync");

    // wait until we get an initial CoordinatorInfo
    let init_info;
    loop {
        println!("Polling for CoordinatorInfo on {:?}...", collection.name());
        match collection.find_all().limit(1).exec() {
            Err(e) => println!("Error: {:?}", e),
            Ok(plan) => {
                if plan.len() > 0 {
                    init_info = Some(plan[0].typed::<CoordinatorInfo>()?);
                    println!("Got CoordinatorInfo {:?}", init_info.as_ref().unwrap());
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

    // start heartbeat timer
    let hctx = HeartbeatCtx::new(hb_record);
    heartbeat_start(hctx.clone())?;

    // wait for execution plan
    loop {
        // XXX subscribe w/ callback instead of polling
        let did = DocumentId::new(&String::from(DEFAULT_DOC_ID))?;
        let doc_result = collection.find_by_id(did)
            .exec();
        if let Err(e) = doc_result {
            println!("Error: {:?}", e);
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
        ditto: make_ditto()?,
        coord_info: None,
        start_time_msec: system_time_msec(),
    };
    println!("Args {:?}", cli);
    bootstrap_peer(&mut pctx, &cli)?;
    Ok(())
}
