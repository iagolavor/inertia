use libp2p::request_response;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{dcutr, identify, relay};

use super::codec::{protocol_stream, request_response_config, JsonCodec};

#[derive(NetworkBehaviour)]
pub struct InertiaBehaviour {
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub request_response: request_response::Behaviour<JsonCodec>,
    pub identify: identify::Behaviour,
}

pub fn build_behaviour(
    key: &libp2p::identity::Keypair,
    relay_client: relay::client::Behaviour,
) -> Result<InertiaBehaviour, Box<dyn std::error::Error + Send + Sync>> {
    Ok(InertiaBehaviour {
        relay_client,
        dcutr: dcutr::Behaviour::new(key.public().to_peer_id()),
        request_response: request_response::Behaviour::new(
            [(protocol_stream(), request_response::ProtocolSupport::Full)],
            request_response_config(),
        ),
        identify: identify::Behaviour::new(identify::Config::new(
            "/inertia/1.0.0".into(),
            key.public(),
        )),
    })
}
