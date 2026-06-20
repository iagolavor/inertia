use serde::Serialize;

/// User-facing connection layers — each is independent and has plain-language labels.
#[derive(Debug, Clone, Serialize)]
pub struct P2pLayers {
    /// `off` | `running`
    pub node: String,
    /// `not_configured` | `unreachable` | `connecting` | `connected`
    pub relay: String,
    /// `offline` | `connecting` | `online`
    pub friends: String,
    /// `idle` | `sending`
    pub sync: String,
    pub friends_online_count: usize,
    pub pending_outbox_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct P2pLayerLabels {
    pub headline: String,
    pub node: String,
    pub relay: String,
    pub friends: String,
    pub sync: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualTone {
    Off,
    Error,
    Warn,
    Idle,
    Online,
}

pub fn build_layers(
    running: bool,
    relay_configured: bool,
    relay_tcp_reachable: Option<bool>,
    relay_connected: bool,
    friends_online_count: usize,
    dial_in_progress: bool,
    pending_outbox_count: usize,
) -> P2pLayers {
    let node = if running { "running" } else { "off" }.to_string();

    let relay = if !relay_configured {
        "not_configured".to_string()
    } else if !running {
        "standby".to_string()
    } else if relay_tcp_reachable == Some(false) {
        "unreachable".to_string()
    } else if relay_connected {
        "connected".to_string()
    } else {
        // TCP probe passed (or still probing) but no libp2p session to relay yet.
        "connecting".to_string()
    };

    let friends = if !running {
        "offline".to_string()
    } else if dial_in_progress {
        "connecting".to_string()
    } else if friends_online_count > 0 {
        "online".to_string()
    } else {
        "offline".to_string()
    };

    let sync = if pending_outbox_count > 0 {
        "sending".to_string()
    } else {
        "idle".to_string()
    };

    P2pLayers {
        node,
        relay,
        friends,
        sync,
        friends_online_count,
        pending_outbox_count,
    }
}

pub fn build_labels(layers: &P2pLayers) -> P2pLayerLabels {
    let node = if layers.node == "running" {
        "P2P node running on this device".to_string()
    } else {
        "P2P not running — restart the API or check Settings".to_string()
    };

    let relay = match layers.relay.as_str() {
        "not_configured" => "Relay: not configured (add one in Settings)".to_string(),
        "standby" => "Relay: configured but P2P is off".to_string(),
        "unreachable" => "Relay: VPS port unreachable — check firewall or address".to_string(),
        "connecting" => {
            "Relay: VPS reachable — establishing P2P connection to relay".to_string()
        }
        "connected" => "Relay: connected (NAT traversal available)".to_string(),
        _ => "Relay: unknown state".to_string(),
    };

    let friends = match layers.friends.as_str() {
        "connecting" => "Friends: connecting to known peers…".to_string(),
        "online" => {
            let n = layers.friends_online_count;
            if n == 1 {
                "Friends: 1 online now".to_string()
            } else {
                format!("Friends: {n} online now")
            }
        }
        _ if layers.node == "off" => "Friends: offline (P2P is off)".to_string(),
        _ => "Friends: none online right now".to_string(),
    };

    let sync = if layers.sync == "sending" {
        let n = layers.pending_outbox_count;
        if n == 1 {
            "Outbox: 1 item waiting to deliver".to_string()
        } else {
            format!("Outbox: {n} items waiting to deliver")
        }
    } else {
        "Outbox: nothing pending".to_string()
    };

    let headline = derive_headline(layers);

    P2pLayerLabels {
        headline,
        node,
        relay,
        friends,
        sync,
    }
}

pub fn visual_tone(layers: &P2pLayers) -> VisualTone {
    if layers.node == "off" {
        return VisualTone::Off;
    }
    if layers.relay == "unreachable" {
        return VisualTone::Error;
    }
    if layers.friends == "online" {
        return VisualTone::Online;
    }
    if layers.friends == "connecting"
        || layers.relay == "connecting"
        || layers.sync == "sending"
    {
        return VisualTone::Warn;
    }
    if layers.relay == "connected" || layers.relay == "not_configured" {
        return VisualTone::Idle;
    }
    VisualTone::Warn
}

fn derive_headline(layers: &P2pLayers) -> String {
    if layers.node == "off" {
        return "P2P off".to_string();
    }
    if layers.relay == "unreachable" {
        return "Relay unreachable".to_string();
    }
    if layers.friends == "online" {
        let n = layers.friends_online_count;
        return if n == 1 {
            "1 friend online".to_string()
        } else {
            format!("{n} friends online")
        };
    }
    if layers.friends == "connecting" {
        return "Connecting to friends…".to_string();
    }
    if layers.sync == "sending" {
        let n = layers.pending_outbox_count;
        return if n == 1 {
            "Sending 1 item…".to_string()
        } else {
            format!("Sending {n} items…")
        };
    }
    if layers.relay == "connecting" {
        return "Connecting to relay…".to_string();
    }
    if layers.relay == "connected" {
        return "Ready — no friends online".to_string();
    }
    if layers.relay == "not_configured" {
        return "Running — no relay set".to_string();
    }
    "P2P idle".to_string()
}

pub fn visual_tone_str(tone: VisualTone) -> &'static str {
    match tone {
        VisualTone::Off => "off",
        VisualTone::Error => "error",
        VisualTone::Warn => "warn",
        VisualTone::Idle => "idle",
        VisualTone::Online => "online",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_connecting_when_tcp_ok_but_no_session() {
        let layers = build_layers(true, true, Some(true), false, 0, false, 0);
        assert_eq!(layers.relay, "connecting");
        let labels = build_labels(&layers);
        assert!(labels.relay.contains("establishing P2P connection"));
        assert_eq!(labels.headline, "Connecting to relay…");
    }

    #[test]
    fn friends_online_headline() {
        let layers = build_layers(true, true, Some(true), true, 2, false, 0);
        assert_eq!(layers.friends, "online");
        let labels = build_labels(&layers);
        assert_eq!(labels.headline, "2 friends online");
        assert_eq!(visual_tone_str(visual_tone(&layers)), "online");
    }
}
