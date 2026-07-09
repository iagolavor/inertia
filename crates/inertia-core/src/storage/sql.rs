use crate::content::{ContentType, DeliveryStatus};

use super::ConnectionState;
use crate::p2p::filter_friend_multiaddrs;

pub(super) const FEED_HISTORY_KEY: &str = "feed_history_enabled";
pub(super) const P2P_LISTEN_PORT_KEY: &str = "p2p_listen_port";
pub(super) const RELAY_MULTIADDRS_KEY: &str = "relay_multiaddrs";
/// Legacy key migrated once on read; no longer written.
pub(super) const RELAY_MULTIADDR_LEGACY_KEY: &str = "relay_multiaddr";
pub(super) const P2P_ANNOUNCE_KEY: &str = "p2p_announce";
pub(super) const WEB_ORIGIN_KEY: &str = "web_origin";

pub(super) fn status_str(status: DeliveryStatus) -> &'static str {
    match status {
        DeliveryStatus::Pending => "pending",
        DeliveryStatus::Sent => "sent",
        DeliveryStatus::Failed => "failed",
        DeliveryStatus::Delivered => "delivered",
        DeliveryStatus::Expired => "expired",
    }
}

pub(super) fn parse_status(s: &str) -> DeliveryStatus {
    match s {
        "failed" => DeliveryStatus::Failed,
        "sent" => DeliveryStatus::Sent,
        "delivered" => DeliveryStatus::Delivered,
        "expired" => DeliveryStatus::Expired,
        _ => DeliveryStatus::Pending,
    }
}

pub(super) fn content_type_str(t: ContentType) -> &'static str {
    match t {
        ContentType::Message => "message",
        ContentType::Post => "post",
        ContentType::Comment => "comment",
        ContentType::ProfileComment => "profile_comment",
    }
}

pub(super) fn parse_content_type(s: &str) -> ContentType {
    match s {
        "post" => ContentType::Post,
        "comment" => ContentType::Comment,
        "profile_comment" => ContentType::ProfileComment,
        _ => ContentType::Message,
    }
}

pub(super) fn connection_state_str(s: ConnectionState) -> &'static str {
    match s {
        ConnectionState::Online => "online",
        ConnectionState::Reachable => "reachable",
        ConnectionState::Offline => "offline",
        ConnectionState::Unreachable => "unreachable",
    }
}

pub(super) fn parse_connection_state(s: &str) -> ConnectionState {
    match s {
        "online" => ConnectionState::Online,
        "reachable" => ConnectionState::Reachable,
        "unreachable" => ConnectionState::Unreachable,
        _ => ConnectionState::Offline,
    }
}

pub(super) fn encode_multiaddrs(addrs: &[String]) -> String {
    serde_json::to_string(addrs).unwrap_or_else(|_| "[]".to_string())
}

pub(super) fn decode_multiaddrs(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

pub(super) fn merge_multiaddr_lists(existing: &[String], new: &[String]) -> Vec<String> {
    let mut out = filter_friend_multiaddrs(existing);
    for addr in filter_friend_multiaddrs(new) {
        if !out.contains(&addr) {
            out.push(addr);
        }
    }
    out
}
