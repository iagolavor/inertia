use std::path::Path;

use libp2p::identity::Keypair;
use tracing::info;

use crate::error::{CoreError, CoreResult};

const P2P_IDENTITY_FILE: &str = "p2p_identity.key";

pub fn load_or_create_keypair(data_dir: &Path) -> CoreResult<Keypair> {
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join(P2P_IDENTITY_FILE);
    if path.exists() {
        let bytes = std::fs::read(&path)?;
        let keypair = Keypair::from_protobuf_encoding(&bytes)
            .map_err(|e| CoreError::P2p(e.to_string()))?;
        info!(path = %path.display(), peer_id = %keypair.public().to_peer_id(), "loaded p2p identity");
        return Ok(keypair);
    }

    let keypair = Keypair::generate_ed25519();
    std::fs::write(
        &path,
        keypair
            .to_protobuf_encoding()
            .map_err(|e| CoreError::P2p(e.to_string()))?,
    )?;
    info!(
        path = %path.display(),
        peer_id = %keypair.public().to_peer_id(),
        "generated new p2p identity"
    );
    Ok(keypair)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p2p_keypair_is_stable_across_loads() {
        let dir = tempfile::tempdir().unwrap();
        let first = load_or_create_keypair(dir.path()).unwrap();
        let second = load_or_create_keypair(dir.path()).unwrap();
        assert_eq!(
            first.public().to_peer_id(),
            second.public().to_peer_id()
        );
    }
}
