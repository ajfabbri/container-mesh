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


#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "container-mesh-coord")]
    coordinator_collection: String,
    #[arg(short, long)]
    coordinator_hostname: String,
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

#[allow(dead_code)]
fn config_transport(ditto: &mut Ditto, cli: &Cli) -> Result<(), Box<dyn Error>> {
    let mut config = TransportConfig::new();
    config.enable_all_peer_to_peer();
    let _ip_addr: std::net::IpAddr = cli.coordinator_hostname.parse()?;
    config.connect.tcp_servers = HashSet::from([cli.coordinator_hostname.clone()]);
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
    finished: Arc<AtomicBool>,
}

// implement new
impl HeartbeatCtx {
    fn new() -> Self {
        HeartbeatCtx {
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

fn bootstrap_peer(cli: &Cli) -> Result<(), Box<dyn Error>> {
    // subscribe to coordinator collection
    println!("Subscribing to coordinator collection {}..", cli.coordinator_collection);
    let ditto = make_ditto().expect("make_ditto");
    let store = ditto.store();
    let _collection = store
        .collection(&cli.coordinator_collection)
        .expect("collection create");
    ditto.start_sync().expect("start_sync");

    // start heartbeat timer
    let hctx = HeartbeatCtx::new();
    heartbeat_start(hctx.clone())?;
    // wait for execution plan
    heartbeat_stop(&hctx);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    println!("Args {:?}", cli);
    bootstrap_peer(&cli)?;
    Ok(())
}
