mod blobs;
mod contacts;
mod expiry;
mod feed;
mod health;
mod identity;
mod inbox;
mod invite;
mod messages;
mod outbox;
mod p2p;
mod posts;
mod profile;
mod settings;

use axum::extract::DefaultBodyLimit;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

const DEFAULT_BODY_LIMIT: usize = 8 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(identity::routes())
        .merge(invite::routes())
        .merge(contacts::routes())
        .merge(inbox::routes())
        .merge(outbox::routes())
        .merge(messages::routes())
        .merge(posts::routes())
        .merge(feed::routes())
        .merge(settings::routes())
        .merge(profile::routes())
        .merge(blobs::routes())
        .merge(p2p::routes())
        .merge(expiry::routes())
        .layer(DefaultBodyLimit::max(DEFAULT_BODY_LIMIT))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
