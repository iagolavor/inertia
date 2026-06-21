use tracing::{info, warn};

use crate::error::{CoreError, CoreResult};
use crate::identity::Identity;

use super::Engine;

impl Engine {
    pub async fn initialize_identity(
        &self,
        display_name: impl Into<String>,
    ) -> CoreResult<Identity> {
        {
            let current = self.identity.read().await;
            if current.is_initialized() {
                return Err(CoreError::ProfileAlreadyExists);
            }
        }

        if self.store.with(|s| s.has_profile()).await? {
            return Err(CoreError::ProfileAlreadyExists);
        }

        let identity = Identity::generate(display_name);
        self.store
            .with_mut(|store| store.create_identity(&identity))
            .await?;
        *self.identity.write().await = identity.clone();
        info!(display_name = %identity.display_name, "identity initialized");
        match self.ensure_p2p_started().await {
            Ok(peer_id) => info!(%peer_id, "auto-started P2P after identity init"),
            Err(e) => warn!(
                error = %e,
                "auto-start P2P after identity init failed; retry via POST /p2p/start or the web app"
            ),
        }
        Ok(identity)
    }

    pub async fn update_profile(
        &self,
        display_name: impl Into<String>,
        bio: impl Into<String>,
    ) -> CoreResult<Identity> {
        let display_name = display_name.into();
        let bio = bio.into();
        if display_name.trim().is_empty() {
            return Err(CoreError::IdentityNotInitialized);
        }

        {
            let current = self.identity.read().await;
            if !current.is_initialized() {
                return Err(CoreError::IdentityNotInitialized);
            }
        }

        self.store
            .with_mut(|store| store.update_identity_profile(&display_name, &bio))
            .await?;

        let mut identity = self.identity.write().await;
        identity.display_name = display_name.trim().to_string();
        identity.bio = bio.trim().to_string();
        Ok(identity.clone())
    }

    pub async fn identity_snapshot(&self) -> Identity {
        self.identity.read().await.clone()
    }
}
