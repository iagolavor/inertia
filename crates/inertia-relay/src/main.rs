use std::path::{Path, PathBuf};
use std::time::Duration;

use libp2p::futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{identify, ping, relay, SwarmBuilder};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:9000";
const IDENTITY_FILE: &str = "relay_identity.key";

#[derive(NetworkBehaviour)]
struct RelayBehaviour {
    relay: relay::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("inertia_relay=info".parse()?))
        .init();

    let data_dir = data_dir_from_env();
    let listen_addr = listen_addr_from_env();
    let keypair = load_or_create_keypair(&data_dir)?;
    let local_peer_id = keypair.public().to_peer_id();

    info!(%local_peer_id, data_dir = %data_dir.display(), "starting inertia-relay");

    let behaviour = RelayBehaviour {
        relay: relay::Behaviour::new(local_peer_id, relay::Config::default()),
        ping: ping::Behaviour::new(ping::Config::new()),
        identify: identify::Behaviour::new(identify::Config::new(
            "/inertia/relay/1.0.0".into(),
            keypair.public(),
        )),
    };

    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|_| behaviour)?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(120)))
        .build();

    swarm.listen_on(listen_addr.parse()?)?;

    loop {
        match swarm.select_next_some().await {
            libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                info!(%address, "relay listening");
                info!(
                    "share with clients: {}/p2p/{}",
                    address,
                    local_peer_id
                );
            }
            libp2p::swarm::SwarmEvent::Behaviour(RelayBehaviourEvent::Relay(
                relay::Event::ReservationReqAccepted {
                    src_peer_id,
                    ..
                },
            )) => {
                info!(%src_peer_id, "relay reservation accepted");
            }
            libp2p::swarm::SwarmEvent::Behaviour(RelayBehaviourEvent::Relay(
                relay::Event::CircuitReqAccepted {
                    src_peer_id,
                    dst_peer_id,
                    ..
                },
            )) => {
                info!(%src_peer_id, %dst_peer_id, "relay circuit established");
            }
            libp2p::swarm::SwarmEvent::Behaviour(RelayBehaviourEvent::Relay(event)) => {
                info!(?event, "relay event");
            }
            libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!(%peer_id, "peer connected to relay");
            }
            libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!(%peer_id, "peer disconnected from relay");
            }
            libp2p::swarm::SwarmEvent::Behaviour(RelayBehaviourEvent::Ping(event)) => {
                if let ping::Event { peer, result: Err(e), .. } = event {
                    warn!(%peer, error = %e, "relay ping failed");
                }
            }
            other => {
                info!(?other, "swarm event");
            }
        }
    }
}

fn data_dir_from_env() -> PathBuf {
    std::env::var("INERTIA_RELAY_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./relay-data"))
}

fn listen_addr_from_env() -> String {
    let raw = std::env::var("INERTIA_RELAY_ADDR").unwrap_or_else(|_| DEFAULT_LISTEN_ADDR.to_string());
    if raw.starts_with('/') {
        return raw;
    }
    if let Some((host, port)) = raw.rsplit_once(':') {
        return format!("/ip4/{host}/tcp/{port}");
    }
    format!("/ip4/{raw}/tcp/9000")
}

fn load_or_create_keypair(data_dir: &Path) -> anyhow::Result<Keypair> {
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join(IDENTITY_FILE);
    if path.exists() {
        let bytes = std::fs::read(&path)?;
        let keypair = Keypair::from_protobuf_encoding(&bytes)?;
        info!(path = %path.display(), "loaded relay identity");
        return Ok(keypair);
    }

    let keypair = Keypair::generate_ed25519();
    std::fs::write(&path, keypair.to_protobuf_encoding()?)?;
    info!(path = %path.display(), "generated new relay identity");
    Ok(keypair)
}
