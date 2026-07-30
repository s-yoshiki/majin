//! Serves the React app, embedded into the binary at compile time.
//!
//! This is what makes the web release a single downloadable file: no Node
//! runtime, no `npm install`, no static directory to deploy alongside it.

use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

/// The path is relative to this crate's manifest directory.
#[derive(Embed)]
#[folder = "../../apps/web/dist"]
struct WebAssets;

const INDEX: &str = "index.html";

/// Fallback route: serves a bundled asset, or `index.html` for client routes.
pub async fn handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { INDEX } else { path };

    if let Some(response) = serve(path) {
        return response;
    }

    // Anything without a file extension is a client-side route, so hand back
    // the SPA shell and let the router sort it out. A missing extension-ful
    // path is a genuine 404.
    if !path.contains('.') {
        if let Some(response) = serve(INDEX) {
            return response;
        }
    }

    (StatusCode::NOT_FOUND, "not found").into_response()
}

fn serve(path: &str) -> Option<Response> {
    let asset = WebAssets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    // Vite fingerprints every asset except the entry document, so those can be
    // cached hard while index.html must always be revalidated.
    let cache_control = if path == INDEX {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    };

    Some(
        (
            [
                (header::CONTENT_TYPE, mime.as_ref()),
                (header::CACHE_CONTROL, cache_control),
            ],
            asset.data.into_owned(),
        )
            .into_response(),
    )
}

/// True when the frontend was actually built into this binary.
pub fn is_embedded() -> bool {
    WebAssets::get(INDEX).is_some()
}
