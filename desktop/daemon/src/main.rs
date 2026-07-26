pub mod config;
pub mod ipc;
pub mod network;
pub mod pair_cmd;
pub mod state;

use clap::Parser;
use config::PairedDeviceStore;
use ipc::IpcServer;
use log::{error, info};
use pair_cmd::PairingCommand;
use state::AuthStateMachine;

#[derive(Parser, Debug)]
#[command(name = "opentapd", version = "1.0.0", author = "OpenTapUnlock Engineering Team")]
#[command(about = "Central Zero-Trust Desktop System Daemon for OpenTapUnlock", long_about = None)]
struct CliArgs {
    /// Run as continuous background system daemon
    #[arg(short, long)]
    daemon: bool,

    /// Launch interactive OOB QR Code pairing session
    #[arg(short, long)]
    pair: bool,

    /// Print current daemon status and list authorized paired devices
    #[arg(short, long)]
    status: bool,

    /// Override local IPC loopback port
    #[arg(long, default_value_t = 30349)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let args = CliArgs::parse();

    if args.pair {
        if let Err(e) = PairingCommand::execute_interactive() {
            eprintln!("[ERROR] Pairing failed: {}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    if args.status {
        println!("====================================================================");
        println!("               OpenTapUnlock Daemon (opentapd) Status               ");
        println!("====================================================================");
        match PairedDeviceStore::load() {
            Ok(store) => {
                let devices = store.all_devices();
                println!("Authorized Mobile Devices in Vault: [{}]", devices.len());
                for (idx, dev) in devices.iter().enumerate() {
                    println!(
                        "  [{}] {} (UUID: {}) - Paired at UTC: {}",
                        idx + 1, dev.device_name, dev.device_uuid, dev.paired_at_utc
                    );
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Could not load device vault: {:?}", e);
            }
        }
        println!("====================================================================");
        return Ok(());
    }

    // Default: Start central daemon
    info!("Starting OpenTapUnlock Desktop Daemon (opentapd v1.0.0)...");
    let state_machine = AuthStateMachine::new();
    let store = PairedDeviceStore::load().unwrap_or_default();
    
    info!("Loaded {} authorized mobile devices from vault.", store.all_devices().len());
    info!("Initializing multi-modal network listeners (BLE GATT, mDNS, mTLS 1.3)...");

    // Spawn IPC Server
    let sm_clone = state_machine.clone();
    let port = args.port;
    tokio::spawn(async move {
        if let Err(e) = IpcServer::start_loopback(port, sm_clone).await {
            error!("IPC Server terminated with error: {:?}", e);
        }
    });

    // Spawn Wi-Fi Zero-Trust Unlock Server (Port 8765)
    let coordinator = network::NetworkCoordinator::new(store.clone(), state_machine.clone());
    tokio::spawn(async move {
        match tokio::net::TcpListener::bind("0.0.0.0:8765").await {
            Ok(listener) => {
                info!("Wi-Fi Zero-Trust Unlock Server actively listening on 0.0.0.0:8765");
                loop {
                    if let Ok((mut socket, peer)) = listener.accept().await {
                        let coord = coordinator.clone();
                        tokio::spawn(async move {
                            use tokio::io::AsyncReadExt;
                            let mut buf = vec![0u8; 4096];
                            if let Ok(n) = socket.read(&mut buf).await {
                                if n > 0 {
                                    info!("Received {} bytes over Wi-Fi from peer: {}", n, peer);
                                    let _ = coord.handle_incoming_packet(&buf[..n]);
                                }
                            }
                        });
                    }
                }
            }
            Err(e) => error!("Failed to bind Wi-Fi unlock server on port 8765: {:?}", e),
        }
    });

    info!("Daemon running actively! Waiting for OS authentication challenges or mobile Triple Taps.");
    info!("Press Ctrl+C to gracefully shutdown daemon.");

    tokio::signal::ctrl_c().await?;
    info!("Received shutdown signal. Closing socket listeners and cleaning up IPC files.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_parsing() {
        let args = CliArgs::parse_from(&["opentapd", "--status"]);
        assert!(args.status);
        assert!(!args.daemon);
        assert_eq!(args.port, 30349);
    }
}
