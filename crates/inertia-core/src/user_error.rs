use crate::error::CoreError;

/// Stable machine-readable codes returned by the HTTP API (`ApiError.code`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    RelayUnreachable,
    InviterOffline,
    P2pNotStarted,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RelayUnreachable => "relay_unreachable",
            Self::InviterOffline => "inviter_offline",
            Self::P2pNotStarted => "p2p_not_started",
        }
    }
}

/// Plain-language message and optional stable code for API clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFacingError {
    pub message: String,
    pub code: Option<ErrorCode>,
}

impl UserFacingError {
    pub fn new(message: impl Into<String>, code: Option<ErrorCode>) -> Self {
        Self {
            message: message.into(),
            code,
        }
    }
}

impl CoreError {
    pub fn user_facing(&self) -> UserFacingError {
        match self {
            CoreError::P2p(msg) if msg == "p2p not started" => UserFacingError::new(
                "P2P is not running on this device. Restart the API or open Settings.",
                Some(ErrorCode::P2pNotStarted),
            ),
            CoreError::Invite(msg)
                if msg.contains("relay VPS port is unreachable")
                    || msg.contains("relay multiaddr is not configured") =>
            {
                UserFacingError::new(
                    "Relay unreachable — check the relay address and VPS firewall in Settings.",
                    Some(ErrorCode::RelayUnreachable),
                )
            }
            CoreError::Invite(msg)
                if msg.contains("relay is not connected yet")
                    || msg.contains("no relay circuit listen addresses") =>
            {
                UserFacingError::new(msg.clone(), Some(ErrorCode::RelayUnreachable))
            }
            CoreError::Invite(msg) if msg.contains("connect to the relay in Settings")
                || msg.contains("no routable P2P addresses") =>
            {
                UserFacingError::new(
                    "Relay unreachable — check the relay address and VPS firewall in Settings.",
                    Some(ErrorCode::RelayUnreachable),
                )
            }
            CoreError::Invite(msg)
                if msg.contains("inviter did not respond")
                    || msg.contains("inviter is not reachable")
                    || msg.contains("inviter has no connection addresses")
                    || msg.contains("inviter must be online") =>
            {
                UserFacingError::new(
                    "Inviter offline — ask them to keep the API running and try again before the link expires.",
                    Some(ErrorCode::InviterOffline),
                )
            }
            CoreError::Invite(msg) => UserFacingError::new(msg.clone(), None),
            CoreError::P2p(msg) => UserFacingError::new(msg.clone(), None),
            CoreError::ProfileAlreadyExists => UserFacingError::new(
                "A profile already exists on this device.",
                None,
            ),
            CoreError::IdentityNotInitialized => UserFacingError::new(
                "Create a profile on this device first.",
                None,
            ),
            CoreError::ContactNotFound(id) => {
                UserFacingError::new(format!("Contact not found: {id}"), None)
            }
            CoreError::ContentNotFound(id) => {
                UserFacingError::new(format!("Content not found: {id}"), None)
            }
            CoreError::Database(e) => UserFacingError::new(format!("Database error: {e}"), None),
            CoreError::Serialization(e) => {
                UserFacingError::new(format!("Data error: {e}"), None)
            }
            CoreError::Crypto(msg) => UserFacingError::new(msg.clone(), None),
            CoreError::Io(e) => UserFacingError::new(format!("IO error: {e}"), None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_p2p_not_started() {
        let err = CoreError::P2p("p2p not started".into());
        let facing = err.user_facing();
        assert_eq!(facing.code, Some(ErrorCode::P2pNotStarted));
        assert!(facing.message.contains("P2P is not running"));
    }

    #[test]
    fn maps_relay_unreachable_on_invite() {
        let err = CoreError::Invite(
            "connect to the relay in Settings before inviting (header must show Relay OK)".into(),
        );
        let facing = err.user_facing();
        assert_eq!(facing.code, Some(ErrorCode::RelayUnreachable));
    }

    #[test]
    fn maps_inviter_offline() {
        let err = CoreError::Invite(
            "inviter did not respond in time — are they online?".into(),
        );
        let facing = err.user_facing();
        assert_eq!(facing.code, Some(ErrorCode::InviterOffline));
    }
}
