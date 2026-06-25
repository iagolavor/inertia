mod blobs;
mod contacts;
mod expiry;
mod feed;
mod health;
mod identity;
mod inbox;
mod invite;
mod media;
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
/// Base64-encoded video + thumb for `POST /posts/video` (~50 MiB video + headroom).
const VIDEO_POST_BODY_LIMIT: usize = 72 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    let video_posts = posts::video_routes().layer(DefaultBodyLimit::max(VIDEO_POST_BODY_LIMIT));

    Router::new()
        .merge(health::routes())
        .merge(identity::routes())
        .merge(invite::routes())
        .merge(contacts::routes())
        .merge(inbox::routes())
        .merge(outbox::routes())
        .merge(messages::routes())
        .merge(posts::routes())
        .merge(video_posts)
        .merge(media::routes())
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
