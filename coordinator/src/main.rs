use clap::Parser;
use dittolive_ditto::error::DittoError;
use dittolive_ditto::prelude::*;
use std::collections::HashSet;
use std::error::Error;
use std::io;
use std::sync::Arc;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long, default_value = "container-mesh-coord")]
    coordinator_collection: String,
    #[arg(short, long, default_value_t = 10)]
    min_msg_delay_msec: u32,
    #[arg(short, long, default_value_t = 500)]
    max_msg_delay_msec: u32,
    #[arg(short, long, default_value_t = 60)]
    test_duration_sec: u32,
}

fn ditto_init() -> Result<Ditto, DittoError> {
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

fn ditto_destroy(ditto: Ditto) {
    ditto.stop_sync();
}


fn config_transport(ditto: &mut Ditto) {
    let mut config = TransportConfig::new();
    config.enable_all_peer_to_peer();
    config.connect.tcp_servers = HashSet::new();
    config.connect.websocket_urls = HashSet::new();
    ditto.set_transport_config(config);
}

fn wait_for_quorum(_cli: &Cli) -> Result<(), DittoError> {
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    println!("Args {:?}", cli);
    let mut ditto = ditto_init()?;
    config_transport(&mut ditto);

    wait_for_quorum(&cli)?;
    ditto_destroy(ditto);
    Ok(())
}
